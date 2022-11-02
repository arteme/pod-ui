mod opts;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use anyhow::*;
use clap::{Args, Command, FromArgMatches};
use core::result::Result::Ok;
use std::ops::Deref;
use futures_util::future::{err, join_all, JoinAll};
use futures_util::FutureExt;
use log::*;
use maplit::*;
use once_cell::sync::Lazy;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use pod_gtk::prelude::*;
use pod_core::midi_io::*;
use pod_core::controller::Controller;
use pod_core::midi::MidiMessage;
use pod_core::model::{Button, Config, Control, MidiQuirks, VirtualSelect};
use pod_core::store::{Event, Store};
use pod_gtk::logic::LogicBuilder;
use pod_gtk::prelude::gtk::gdk;
use crate::opts::*;

const MIDI_OUT_CHANNEL_CAPACITY: usize = 512;
const CLOSE_QUIET_DURATION_MS: u64 = 1000;

#[derive(Clone, Debug)]
pub enum ProgramSelect {
    ManualMode,
    Tuner,
    Program(u16)
}

impl From<u16> for ProgramSelect {
    fn from(v: u16) -> Self {
        match v {
            999 => ProgramSelect::Tuner,
            998 => ProgramSelect::ManualMode,
            v => ProgramSelect::Program(v)
        }
    }
}

impl Into<u16> for ProgramSelect {
    fn into(self) -> u16 {
        match self {
            ProgramSelect::Tuner => 999,
            ProgramSelect::ManualMode => 998,
            ProgramSelect::Program(v) => v
        }
    }
}

#[derive(Clone, Debug)]
pub enum Program {
    EditBuffer,
    Current,
    Number(usize),
    All
}

#[derive(Clone, Debug)]
pub enum AppEvent {
    /// Data was sent to a MIDI out port
    MidiTx,
    /// Data was received from a MIDI in port
    MidiRx,

    MidiIn(Vec<u8>),
    MidiOut(Vec<u8>),

    MidiMsgIn(MidiMessage),
    MidiMsgOut(MidiMessage),

    ProgramChange(ProgramSelect),
    Load(Program),
    Store(Program),

    Shutdown,
    Quit,
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

    /*
    let config_changed = match (config, *state.config.read().unwrap()) {
        (Some(a), b) => { *a != *b }
        _ => { false }
    };
    if config_changed {
        // config changed, update config & edit buffer
        let config = config.unwrap();
        {
            let mut c = state.config.write().unwrap();
            *c = config;
        }

        info!("Initiating module for config {:?}", &config.name);

        state.interface = init_module(config).unwrap();

        state.edit_buffer.store(state.interface.edit_buffer.clone());
        state.dump.store(state.interface.dump.clone());

        info!("Installing config {:?}", &config.name);

        state.ui_event_tx.send(UIEvent::NewEditBuffer);
    }
     */

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
            app_event_tx.send(AppEvent::ProgramChange(v.into()));
        })
        .on("load_button")
        .run(move |_,_,_,app_event_tx| {
            app_event_tx.send(AppEvent::Load(Program::EditBuffer));
        })
        .on("load_patch_button")
        .run(move |_,_,_,app_event_tx| {
            app_event_tx.send(AppEvent::Load(Program::Current));
        })
        .on("load_all_button")
        .run(move |_,_,_,app_event_tx| {
            app_event_tx.send(AppEvent::Load(Program::All));
        })
        .on("store_button")
        .run(move |_,_,_,app_event_tx| {
            app_event_tx.send(AppEvent::Store(Program::EditBuffer));
        })
        .on("store_patch_button")
        .run(move |_,_,_,app_event_tx| {
            app_event_tx.send(AppEvent::Store(Program::Current));
        })
        .on("store_all_button")
        .run(move |_,_,_,app_event_tx| {
            app_event_tx.send(AppEvent::Store(Program::All));
        });

    Ok(())
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

    let help_text = generate_help_text()?;
    let cli = Command::new("Pod UI")
        .version(VERSION.as_str())
        .after_help(&*help_text)
        .after_long_help(&*help_text);

    let cli = Opts::augment_args(cli);
    let opts: Opts = Opts::from_arg_matches(&cli.get_matches())?;
    drop(help_text);

    let (app_event_tx, mut app_event_rx) = broadcast::channel::<AppEvent>(MIDI_OUT_CHANNEL_CAPACITY);
    let state = State {
        midi_in_name: None,
        midi_in_cancel: None,
        midi_in_handle: None,
        midi_out_name: None,
        midi_out_cancel: None,
        midi_out_handle: None,
        midi_channel_num: 0,
        app_event_tx: app_event_tx.clone(),
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
    let (ui_tx, ui_rx) = glib::MainContext::channel::<String>(glib::PRIORITY_DEFAULT);

    tokio::spawn(async move {
        /*
        tokio::select! {
            msg = app_event_rx.recv() => {
                // app event
                msg.is
                let msg = match msg {
                    Ok()
                }

                match msg {
                    Ok(AppEvent::Shutdown) => {
                        ui_tx.send("".into());
                    }
                    _ => {}
                }

                ui_tx.send("hello".into());
            }
        }*/
        loop {
            if let Ok(msg) = app_event_rx.recv().await {
                match msg {
                    // message conversion
                    AppEvent::MidiIn(bytes) => {
                        let msg = MidiMessage::from_bytes(bytes).unwrap();
                        app_event_tx.send(AppEvent::MidiMsgIn(msg));
                    }
                    AppEvent::MidiMsgOut(msg) => {
                        let bytes = MidiMessage::to_bytes(&msg);
                        app_event_tx.send(AppEvent::MidiOut(bytes));
                    }

                    _ => {
                        error!("Unhandled app event: {:?}", msg);
                    }
                }

            }

        }

    });

    //let c = glib::MainContext::default();
    //c.block_on()

    let mut ui_rx = ui_controller.subscribe();

    glib::MainContext::default()
        .spawn_local(async move {
            loop {
                let name = match ui_rx.recv().await {
                    Ok(Event { key, .. }) => { key }
                    Err(_) => {
                        continue;
                    }
                };
                println!("got {}", name);

                let vec = ui_callbacks.get_vec(&name);
                match vec {
                    None => { warn!("No GUI callback for '{}'", &name); },
                    Some(vec) => for cb in vec {
                        cb()
                    }
                }
                let val = ui_controller.get(&name).unwrap();
                animate(&ui_objects, &name, val);
            }
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

    // show the window and do init stuff...
    window.show_all();
    window.resize(1, 1);

    debug!("starting gtk main loop");
    gtk::main();
    debug!("end of gtk main loop");

    Ok(())
}