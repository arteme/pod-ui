mod opts;
mod registry;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use anyhow::*;
use clap::{Args, Command, FromArgMatches};
use core::result::Result::Ok;
use std::ops::Deref;
use futures_util::future::{join_all, JoinAll};
use futures_util::FutureExt;
use log::*;
use maplit::*;
use once_cell::sync::Lazy;
use tokio::sync::{broadcast, oneshot};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use pod_gtk::prelude::*;
use pod_core::midi_io::*;
use pod_core::context::Ctx;
use pod_core::controller::*;
use pod_core::event::{AppEvent, Buffer, BufferLoadEvent, BufferStoreEvent, ControlChangeEvent, EventSenderExt, Origin, ProgramChangeEvent};
use pod_core::generic::{cc_handler, pc_handler};
use pod_core::midi::MidiMessage;
use pod_core::model::{Button, Config, Control, MidiQuirks, VirtualSelect};
use pod_core::store::{Event, Store};
use pod_core::stack::ControllerStack;
use pod_gtk::logic::LogicBuilder;
use pod_gtk::prelude::gtk::gdk;
use crate::opts::*;
use crate::registry::{init_module, InitializedInterface, register_module};

const MIDI_OUT_CHANNEL_CAPACITY: usize = 512;
const CLOSE_QUIET_DURATION_MS: u64 = 1000;


#[derive(Clone)]
pub enum UIEvent {
    NewEditBuffer(Option<Ctx>),
}

pub struct State {
    pub midi_in_name: Option<String>,
    pub midi_in_cancel: Option<oneshot::Sender<()>>,
    pub midi_in_handle: Option<JoinHandle<()>>,

    pub midi_out_name: Option<String>,
    pub midi_out_cancel: Option<oneshot::Sender<()>>,
    pub midi_out_handle: Option<JoinHandle<()>>,

    pub midi_channel_num: u8,

    pub app_event_tx: broadcast::Sender<AppEvent>,

    pub config: Option<&'static Config>,
    pub interface: Option<InitializedInterface>,
}

static UI_CONTROLS: Lazy<HashMap<String, Control>> = Lazy::new(|| {
    convert_args!(hashmap!(
        "program" => VirtualSelect::default(),
        "program:prev" => VirtualSelect::default(),
        "program_num" => VirtualSelect::default(),
        "load_button" => Button::default(),
        "load_patch_button" => Button::default(),
        "load_all_button" => Button::default(),
        "store_button" => Button::default(),
        "store_patch_button" => Button::default(),
        "store_all_button" => Button::default(),

        /// Set if device config contains DeviceFlags::MANUAL_MODE
        "manual_mode_present" => VirtualSelect::default(),
    ))
});


static VERSION: Lazy<String> = Lazy::new(|| {
    let version = env!("GIT_VERSION");
    let features: Vec<&str> = vec![
        if cfg!(feature = "winrt") { Some("winrt") } else { None }
    ].into_iter().flatten().collect();

    if features.is_empty() {
        version.to_string()
    } else {
        format!("{} ({})", version, features.join(","))
    }
});

fn sentry_set_midi_tags(in_name: Option<&String>, out_name: Option<&String>) {
    sentry::configure_scope(|scope| {
        scope.set_tag("midi.in",
                      in_name.unwrap_or(&"-".to_string()));
        scope.set_tag("midi.out",
                      out_name.unwrap_or(&"-".to_string()));
    })
}

fn sentry_set_device_tags(detected_name: &String, detected_ver: &String, config_name: &String) {
    sentry::configure_scope(|scope| {
        scope.set_tag("device.name", detected_name);
        scope.set_tag("device.ver", detected_ver);
        scope.set_tag("device.config", config_name);
    })
}

pub fn midi_in_out_stop(state: &mut State) -> JoinAll<JoinHandle<()>> {
    state.midi_in_cancel.take().map(|cancel| cancel.send(()));
    state.midi_out_cancel.take().map(|cancel| cancel.send(()));

    /* TODO: this should one day be 'async fn midi_in_out_stop' so that we
             can wait on the MIDI in/out threads stopping, but for now,
             State is not Send, so we can't really schedule this in a
             separate thread. For now we assume that MIDI in/out threads stop
             "very soon after cancel is signalled", which should be good
             enough around the settings dialog user interaction...
    */

    let handles = state.midi_in_handle.take().into_iter()
        .chain(state.midi_out_handle.take().into_iter());
    join_all(handles)
}

pub fn midi_in_out_start(state: &mut State,
                         midi_in: Option<MidiIn>, midi_out: Option<MidiOut>,
                         midi_channel: u8, quirks: MidiQuirks) {
    if midi_in.is_none() || midi_out.is_none() {
        warn!("Not starting MIDI because in/out is None");
        state.midi_in_name = None;
        state.midi_in_cancel = None;
        state.midi_out_name = None;
        state.midi_out_cancel = None;
        state.midi_channel_num = 0;
        /*state.ui_event_tx.send(UIEvent::NewMidiConnection)
            .map_err(|err| warn!("Cannot send UIEvent: {}", err))
            .unwrap();

         */
        sentry_set_midi_tags(state.midi_in_name.as_ref(), state.midi_out_name.as_ref());
        return;
    }

    let mut midi_in = midi_in.unwrap();
    let mut midi_out = midi_out.unwrap();

    let (in_cancel_tx, mut in_cancel_rx) = oneshot::channel::<()>();
    let (out_cancel_tx, mut out_cancel_rx) = oneshot::channel::<()>();

    state.midi_in_name = Some(midi_in.name.clone());
    state.midi_in_cancel = Some(in_cancel_tx);

    state.midi_out_name = Some(midi_out.name.clone());
    state.midi_out_cancel = Some(out_cancel_tx);

    state.midi_channel_num = midi_channel;
    /*
    state.ui_event_tx.send(UIEvent::NewMidiConnection)
        .map_err(|err| warn!("Cannot send UIEvent: {}", err))
        .unwrap();

     */
    sentry_set_midi_tags(state.midi_in_name.as_ref(), state.midi_out_name.as_ref());

    // midi in
    let midi_in_handle =
        tokio::spawn({
            let app_event_tx = state.app_event_tx.clone();
            let mut in_cancel_rx = in_cancel_rx.fuse();

            async move {
                let id = thread::current().id();
                let mut close_quiet_duration: Option<Duration> = None;

                info!("MIDI in thread {:?} start", id);
                loop {
                    tokio::select! {
                        msg = midi_in.recv() => {
                            match msg {
                                Some(bytes) => {
                                    app_event_tx.send(AppEvent::MidiIn(bytes));
                                    app_event_tx.send(AppEvent::MidiRx);
                                }
                                _ => {}
                            }
                        }
                        _ = &mut in_cancel_rx => {
                            if quirks.contains(MidiQuirks::MIDI_CLOSE_QUIET_TIMEOUT) {
                                debug!("close_quiet_duration set!");
                                close_quiet_duration = Some(Duration::from_millis(CLOSE_QUIET_DURATION_MS));
                            } else {
                                break;
                            }
                        }
                        _ = async {
                            if let Some(d) = close_quiet_duration {
                                sleep(d).await
                            } else {
                                std::future::pending::<()>().await
                            }
                        } => {
                            break;
                        }
                    }
                }
                midi_in.close();
                info!("MIDI in thread {:?} finish", id);
            }
        });

    // midi out
    let midi_out_handle =
        tokio::spawn({
            let app_event_tx = state.app_event_tx.clone();
            let mut app_event_rx = state.app_event_tx.subscribe();
            let mut out_cancel_rx = out_cancel_rx.fuse();

            async move {
                let id = thread::current().id();
                info!("MIDI out thread {:?} start", id);
                loop {
                    tokio::select! {
                        msg = app_event_rx.recv() => {
                            match msg {
                                Ok(AppEvent::MidiOut(bytes)) => {
                                    midi_out.send(&bytes)
                                    .unwrap_or_else(|e| error!("MIDI OUT thread tx error: {}", e));
                                    app_event_tx.send(AppEvent::MidiTx);
                                }
                                Err(err) => {
                                    error!("MIDI OUT thread rx error: {:?}", err);
                                }
                                _ => {}
                            }
                        }
                        _ = &mut out_cancel_rx => {
                            midi_out.close();
                            break;
                        }
                    }
                }
                midi_out.close();
                info!("MIDI out thread {:?} finish", id);
            }
        });

    state.midi_in_handle = Some(midi_in_handle);
    state.midi_out_handle = Some(midi_out_handle);
}

pub fn set_midi_in_out(state: &mut State, midi_in: Option<MidiIn>, midi_out: Option<MidiOut>,
                       midi_channel: u8, config: Option<&'static Config>) -> bool {
    midi_in_out_stop(state);

    let config_changed = match (config, state.config.unwrap()) {
        (Some(a), b) => { *a != *b }
        _ => { false }
    };
    if config_changed {
        // config changed, update config & edit buffer
        let config = config.unwrap();
        state.config.replace(config);

        info!("Initiating module for config {:?}", &config.name);

        state.interface = init_module(config)
            .map_err(|err| error!("Failed to initialize config {:?}: {}", config.name, err))
            .ok();

        info!("Installing config {:?}", &config.name);

        state.app_event_tx.send_or_warn(AppEvent::NewConfig);
    }


    //let quirks = state.config.read().unwrap().midi_quirks;
    let quirks = MidiQuirks::empty();
    midi_in_out_start(state, midi_in, midi_out, midi_channel, quirks);

    // we assume that something changed -- either the config or the midi settings
    // so signal a new device ping!
    //state.ui_event_tx.send(UIEvent::NewDevice);

    //config_changed
    false
}

fn wire_ui_controls(
    controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks,
    app_event_tx: broadcast::Sender<AppEvent>
) -> Result<()> {
    wire(controller.clone(), objs, callbacks)?;

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        .data(app_event_tx.clone())
        .on("program")
        .run(move |v, _, _, app_event_tx| {
            let e = ProgramChangeEvent { program: v.into(), origin: Origin::UI };
            app_event_tx.send(AppEvent::ProgramChange(e));
        })
        .on("load_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: Origin::UI };
            app_event_tx.send(AppEvent::Load(e));
        })
        .on("load_patch_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferLoadEvent { buffer: Buffer::Current, origin: Origin::UI };
            app_event_tx.send(AppEvent::Load(e));
        })
        .on("load_all_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferLoadEvent { buffer: Buffer::All, origin: Origin::UI };
            app_event_tx.send(AppEvent::Load(e));
        })
        .on("store_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferStoreEvent { buffer: Buffer::EditBuffer, origin: Origin::UI };
            app_event_tx.send(AppEvent::Store(e));
        })
        .on("store_patch_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferStoreEvent { buffer: Buffer::Current, origin: Origin::UI };
            app_event_tx.send(AppEvent::Store(e));
        })
        .on("store_all_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferStoreEvent { buffer: Buffer::All, origin: Origin::UI };
            app_event_tx.send(AppEvent::Store(e));
        });

    Ok(())
}

/*
struct StackData {
    pub stack: ControllerStack,
    pub objects: ObjectList,
    pub callbacks: Callbacks,
    pub n: usize,
    pub controllers_list: Vec<Arc<Mutex<Controller>>>,
    pub objects_list: Vec<ObjectList>,
    pub callbacks_list: Vec<Callbacks>
}

impl StackData {
    pub fn new() -> Self {
        Self {
            stack: ControllerStack::new(),
            objects: ObjectList::default(),
            callbacks: Callbacks::default(),
            n: 0,
            controllers_list: vec![],
            objects_list: vec![],
            callbacks_list: vec![]
        }
    }

    pub fn push(&mut self, controller: Arc<Mutex<Controller>>, obj: ObjectList, cb: Callbacks) {
        self.controllers_list.push(controller.clone());
        self.objects_list.push(obj.clone());
        self.callbacks_list.push(cb.clone());
        self.n += 1;
        self.stack.add(controller);
        self.objects = &self.objects + &obj;
        self.callbacks = &self.callbacks + &cb;
    }

}
*/

async fn controller_rx_handler<F>(rx: &mut broadcast::Receiver<Event<String>>,
                                  controller: &Arc<Mutex<Controller>>,
                                  objs: &ObjectList, callbacks: &Callbacks,
                                  f: F) -> bool
    where F: Fn(String, u16, u8) -> ()
{
    let (name, origin) = match rx.recv().await {
        Ok(Event { key, origin, .. }) => { (key, origin) }
        Err(RecvError::Closed) => { return true; }
        Err(RecvError::Lagged(_)) => { return false; }
    };
    println!("got {}", name);

    let vec = callbacks.get_vec(&name);
    match vec {
        None => { warn!("No UI callback for '{}'", &name); },
        Some(vec) => for cb in vec {
            cb()
        }
    }
    let value = controller.get(&name).unwrap();
    animate(&objs, &name, value);

    f(name, value, origin);
    false
}

async fn controller_rx_handler_nop(rx: &mut broadcast::Receiver<Event<String>>,
                                   controller: &Arc<Mutex<Controller>>,
                                   objs: &ObjectList, callbacks: &Callbacks) -> bool {
    controller_rx_handler(rx, controller, objs, callbacks, |_,_,_| {}).await
}

fn start_controller_rx<F>(controller: Arc<Mutex<Controller>>,
                          objs: ObjectList, callbacks: Callbacks,
                          f: F)
    where F: Fn(String, u16, u8) -> () + 'static
{
    let controller = controller.clone();
    let (tx, mut rx) = broadcast::channel::<Event<String>>(MIDI_OUT_CHANNEL_CAPACITY);
    controller.broadcast(Some(tx));

    glib::MainContext::default()
        .spawn_local(async move {
            loop {
                let stop = controller_rx_handler(&mut rx, &controller, &objs, &callbacks, &f).await;
                if stop { break; }
            }
        });
}


#[tokio::main]
async fn main() -> Result<()> {
    let _guard = sentry::init((option_env!("SENTRY_DSN"), sentry::ClientOptions {
        release: Some(VERSION.as_str().into()),
        ..Default::default()
    }));
    let sentry_enabled = _guard.is_enabled();
    simple_logger::init()?;

    // TODO: register modules
    register_module(pod_mod_xt::module());

    let help_text = generate_help_text()?;
    let cli = Command::new("Pod UI")
        .version(VERSION.as_str())
        .after_help(&*help_text)
        .after_long_help(&*help_text);

    let cli = Opts::augment_args(cli);
    let opts: Opts = Opts::from_arg_matches(&cli.get_matches())?;
    drop(help_text);

    let (app_event_tx, mut app_event_rx) = broadcast::channel::<AppEvent>(MIDI_OUT_CHANNEL_CAPACITY);
    let (new_config_tx, mut new_config_rx) = broadcast::channel::<()>(1);
    let state = State {
        midi_in_name: None,
        midi_in_cancel: None,
        midi_in_handle: None,
        midi_out_name: None,
        midi_out_cancel: None,
        midi_out_handle: None,
        midi_channel_num: 0,
        app_event_tx: app_event_tx.clone(),
        config: None,
        interface: None,
    };

    // autodetect
    // TODO: here?

    // UI

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    let ui = gtk::Builder::from_string(include_str!("ui.glade"));
    let ui_objects = ObjectList::new(&ui);
    let mut ui_callbacks = Callbacks::new();
    let ui_controller = Arc::new(Mutex::new(Controller::new((*UI_CONTROLS).clone())));
    wire_ui_controls(ui_controller.clone(), &ui_objects, &mut ui_callbacks,
                     app_event_tx.clone())?;
    wire_settings_dialog(state.clone(), &ui);
    wire_panic_indicator(state.clone());

    let title = format!("POD UI {}", &*VERSION);

    let window: gtk::Window = ui.object("ui_win").unwrap();
    window.set_title(&title);
    window.connect_delete_event({
        let app_event_tx = app_event_tx.clone();
        move |_, _| {
            info!("Shutting down...");
            app_event_tx.send(AppEvent::Shutdown);
            Inhibit(true)
        }
    });

    let css = gtk::CssProvider::new();
    css.load_from_data(include_str!("default.css").as_bytes())
        .unwrap_or_else(|e| error!("Failed to load default CSS: {}", e.message()));
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Error initializing GTK CSS provider"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    // HERE
    let (ui_tx, ui_rx) = glib::MainContext::channel::<UIEvent>(glib::PRIORITY_DEFAULT);
    let mut ctx: Option<Ctx> = None;

    tokio::spawn({
        let app_event_tx = app_event_tx.clone();

        async move {
            loop {
                let msg = match app_event_rx.recv().await {
                    Ok(msg) => { msg }
                    Err(RecvError::Closed) => {
                        info!("App event bus closed");
                        return;
                    }
                    Err(RecvError::Lagged(n)) => {
                        error!("App event bus lagged: {}", n);
                        continue;
                    }
                };

                match &msg {
                    // message conversion
                    AppEvent::MidiIn(bytes) => {
                        // todo: do not clone
                        let msg = MidiMessage::from_bytes(bytes.clone()).unwrap();
                        app_event_tx.send(AppEvent::MidiMsgIn(msg));
                    }
                    AppEvent::MidiMsgOut(msg) => {
                        let bytes = MidiMessage::to_bytes(&msg);
                        app_event_tx.send(AppEvent::MidiOut(bytes));
                    }

                    // control change
                    AppEvent::ControlChange(cc) => {
                        let Some(ctx) = &ctx else {
                            warn!("CC event {:?} without context", cc);
                            continue;
                        };
                        cc_handler(&ctx, cc);
                    }

                    // program change
                    AppEvent::ProgramChange(pc) => {
                        let Some(ctx) = &ctx else {
                            warn!("CC event {:?} without context", pc);
                            continue;
                        };
                        pc_handler(&ctx, pc);

                    }

                    AppEvent::NewConfig => {
                        ui_tx.send(UIEvent::NewEditBuffer(ctx.clone()));
                    }
                    AppEvent::NewCtx(c) => {
                        ctx.replace(c.clone());
                    }

                    _ => {
                        error!("Unhandled app event: {:?}", msg);
                    }
                }
            }

        }
    });

    // run UI controller callback on the GTK thread
    glib::MainContext::default()
        .spawn_local({
            let ui_controller = ui_controller.clone();
            let (ui_tx, mut ui_rx) = broadcast::channel::<Event<String>>(MIDI_OUT_CHANNEL_CAPACITY);
            ui_controller.broadcast(Some(ui_tx));

            async move {
                loop {
                    let stop = controller_rx_handler_nop(
                        &mut ui_rx, &ui_controller, &ui_objects, &ui_callbacks
                    ).await;
                    if stop { break; }
                }
            }
        });

    // run UI event handling on the GTK thread
    ui_rx.attach(None, move |event| {

        match event {
            UIEvent::NewEditBuffer(ctx) => {
                if let Some(ctx) = &ctx {
                    // detach old controller from app events
                    ctx.controller.broadcast(None);
                }

                let interface = state.interface.as_ref().unwrap();

                let controller = interface.edit_buffer.lock().unwrap().controller();
                let objs = interface.objects.clone();
                let callbacks = interface.callbacks.clone();

                {
                    let app_event_tx = app_event_tx.clone();
                    start_controller_rx(
                        controller.clone(), objs, callbacks,
                        move |name, value, origin| {
                            let origin = match origin {
                                pod_core::config::MIDI => Origin::MIDI,
                                pod_core::config::GUI => Origin::UI,
                                _ => {
                                    error!("Unknown origin");
                                    return;
                                }
                            };

                            let e = ControlChangeEvent { name, value, origin };
                            app_event_tx.send_or_warn(AppEvent::ControlChange(e));
                        }
                    );
                }

                let ctx = Ctx {
                    config: state.config.unwrap(),
                    controller,
                    edit: interface.edit_buffer.clone(),
                    dump: interface.dump.clone(),
                    ui_controller: ui_controller.clone(),
                    app_event_tx: app_event_tx.clone()
                };
                app_event_tx.send_or_warn(AppEvent::NewCtx(ctx));

            }
        }



            //device_box.foreach(|w| device_box.remove(w));
            //device_box.add(&state.interface.widget);

            /*
            {
                // Another ugly hack: to initialize the UI, it is not enough
                // to animate() the init_controls, since the controls do
                // emit other (synthetic or otherwise) animate() calls in their
                // wiring. So, we both animate() as part of init_module()
                // (needed to hide most controls that hide before first show)
                // and defer an init_module_controls() call that needs to happen
                // after the rx is subscribed to again!
                let config = state.config.clone();
                let edit_buffer = edit_buffer.load().clone();
                glib::idle_add_local_once(move || {
                    let config = config.read().unwrap();
                    let edit_buffer = edit_buffer.lock().unwrap();
                    init_module_controls(&config, &edit_buffer)
                        .unwrap_or_else(|err| error!("{}", err));
                });
            }

            // Update UI from state after device change
            update_ui_from_state(&state, &mut ui_controller.lock().unwrap());

            let grid = ui.object::<gtk::Grid>("program_grid").unwrap();
            ObjectList::from_widget(&grid)
                .objects_by_type::<ProgramGrid>()
                .for_each(|p| {
                    grid.remove(p);
                    // This instance of ProgramGrid gets dropped, but the
                    // ad-hoc signalling using "group-changed" still sees
                    // its widgets as part of the radio group (not dropped
                    // immediately?) and there will be no final signal to
                    // reset the group/signal handlers to remove the ones
                    // that became invalid.
                    // TODO: add "on destroy" clean up to wired RadioButtons
                    //       as a fix? In  the meantime, this hack will do...
                    p.join_radio_group(Option::<&gtk::RadioButton>::None);
                });

            let program_num = state.config.read().unwrap().program_num;
            let g = ProgramGrid::new(program_num);
            grid.attach(&g, 0, 1, 2, 18);
            g.show_all();

            // join the main program radio group
            let r = ui.object::<gtk::RadioButton>("program").unwrap();
            g.join_radio_group(Some(&r));
            r.emit_by_name::<()>("group-changed", &[]);

            // show "open" button in the titlebar?
            let open_button = ui.object::<gtk::Button>("open_button").unwrap();
            if g.num_pages() > 1 {
                open_button.show();
            } else {
                open_button.hide();
            }

            program_grid.store(Arc::new(g));

            make_window_smaller(window.clone());

            let s = state.app_event_tx.clone();
             */

            Continue(true)
        });


    // gtk
    /*
    ui_rx.attach(None, move |name| {
        if name.is_empty() {
            // quit
            gtk::main_quit();
            return Continue(false);
        }


        Continue(true)
    });

     */

    glib::timeout_add_local_once(
        Duration::from_millis(5000),
        || {
        });

    // show the window and do init stuff...
    window.show_all();
    window.resize(1, 1);

    debug!("starting gtk main loop");
    gtk::main();
    debug!("end of gtk main loop");

    Ok(())
}