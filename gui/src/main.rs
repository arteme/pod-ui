mod opts;
mod registry;
mod settings;
mod util;
mod panic;
mod widgets;
mod autodetect;
mod check;
mod icon;

use std::collections::HashMap;
use std::sync::{Arc, atomic, Mutex};
use std::time::{Duration, Instant};
use anyhow::*;
use clap::{Args, Command, FromArgMatches};
use core::result::Result::Ok;
use std::env;
use std::rc::Rc;
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
use pod_core::event::*;
use pod_core::dispatch::*;
use pod_core::dump::ProgramsDump;
use pod_core::midi::MidiMessage;
use pod_core::model::{Button, Config, Control, DeviceFlags, MidiQuirks, VirtualSelect};
use pod_core::program_id_string;
use pod_gtk::logic::LogicBuilder;
use pod_gtk::prelude::gtk::gdk;
use crate::check::{current_platform, new_release_check};
use crate::icon::set_app_icon;
use crate::opts::*;
use crate::panic::*;
use crate::registry::*;
use crate::settings::*;
use crate::util::{next_thread_id, SenderExt as SenderExt2};
use crate::widgets::*;
use crate::widgets::templated::Templated;

const MIDI_OUT_CHANNEL_CAPACITY: usize = 512;
const CLOSE_QUIET_DURATION_MS: u64 = 1000;


#[derive(Clone, Debug)]
pub enum UIEvent {
    DeviceDetected(DeviceDetectedEvent),
    NewMidiConnection,
    NewConfig,
    MidiTx,
    MidiRx,
    Panic,
    Modified(usize, bool),
    Name(usize, String),
    Notification(String, Option<String>),
    Shutdown,
    Quit
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
    pub ui_event_tx: glib::Sender<UIEvent>,

    pub config: Option<&'static Config>,
    pub detected: Option<DeviceDetectedEvent>,
}

static UI_CONTROLS: Lazy<HashMap<String, Control>> = Lazy::new(|| {
    convert_args!(hashmap!(
        "midi_channel" => VirtualSelect::default(),
        "program" => VirtualSelect::default(),
        "program:prev" => VirtualSelect::default(),

        "load_button" => Button::default(),
        "load_patch_button" => Button::default(),
        "load_all_button" => Button::default(),
        "store_button" => Button::default(),
        "store_patch_button" => Button::default(),
        "store_all_button" => Button::default(),

        // Set if device config contains DeviceFlags::MANUAL_MODE
        "manual_mode_present" => VirtualSelect::default(),
        // This will be always-on for now
        "tuner_present" => VirtualSelect::default(),
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

    let handles = state.midi_in_handle.take().into_iter()
        .chain(state.midi_out_handle.take().into_iter());
    join_all(handles)
}

pub fn midi_in_out_start(state: &mut State,
                         midi_in: Option<MidiIn>, midi_out: Option<MidiOut>,
                         midi_channel: u8, quirks: MidiQuirks,
                         config_changed: bool) {

    let notify = |state: &mut State| {
        sentry_set_midi_tags(state.midi_in_name.as_ref(), state.midi_out_name.as_ref());

        let e = NewConfigEvent {
            midi_changed: true,
            midi_channel: state.midi_channel_num,
            config_changed
        };
        state.app_event_tx.send_or_warn(AppEvent::NewConfig(e));
    };

    if midi_in.is_none() || midi_out.is_none() {
        warn!("Not starting MIDI because in/out is None");
        state.midi_in_name = None;
        state.midi_in_cancel = None;
        state.midi_out_name = None;
        state.midi_out_cancel = None;
        state.midi_channel_num = midi_channel;
        notify(state);
        return;
    }

    let mut midi_in = midi_in.unwrap();
    let mut midi_out = midi_out.unwrap();

    let (in_cancel_tx, in_cancel_rx) = oneshot::channel::<()>();
    let (out_cancel_tx, out_cancel_rx) = oneshot::channel::<()>();

    state.midi_in_name = Some(midi_in.name.clone());
    state.midi_in_cancel = Some(in_cancel_tx);

    state.midi_out_name = Some(midi_out.name.clone());
    state.midi_out_cancel = Some(out_cancel_tx);

    state.midi_channel_num = midi_channel;

    notify(state);

    // midi in
    let midi_in_handle =
        tokio::spawn({
            let app_event_tx = state.app_event_tx.clone();
            let ui_event_tx = state.ui_event_tx.clone();
            let mut in_cancel_rx = in_cancel_rx.fuse();

            async move {
                let id = next_thread_id();
                let mut close_quiet_duration: Option<Duration> = None;

                info!("MIDI in thread {:?} start", id);
                loop {
                    tokio::select! {
                        msg = midi_in.recv() => {
                            match msg {
                                Some(bytes) => {
                                    app_event_tx.send_or_warn(AppEvent::MidiIn(bytes));
                                    ui_event_tx.send_or_warn(UIEvent::MidiRx);
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
            let ui_event_tx = state.ui_event_tx.clone();
            let mut app_event_rx = state.app_event_tx.subscribe();
            let mut out_cancel_rx = out_cancel_rx.fuse();

            async move {
                let id = next_thread_id();
                info!("MIDI out thread {:?} start", id);
                loop {
                    tokio::select! {
                        msg = app_event_rx.recv() => {
                            match msg {
                                Ok(AppEvent::MidiOut(bytes)) => {
                                    midi_out.send(&bytes)
                                    .unwrap_or_else(|e| error!("MIDI OUT thread tx error: {}", e));
                                    ui_event_tx.send_or_warn(UIEvent::MidiTx);
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
    if state.midi_in_cancel.is_some() || state.midi_out_cancel.is_some() {
        error!("Midi still running when entering send_midi_in_out");
        // Not sure if we ever end up in this situation anymore,
        // let's just register it to sentry for not
        sentry::capture_message("Midi still running when entering send_midi_in_out",
                                sentry::Level::Error);
        //midi_in_out_stop(state);
    }

    let config_changed = match (config, state.config) {
        (Some(a), Some(b)) => { *a != *b }
        (Some(_), None) => { true }
        _ => { false }
    };
    if config_changed {
        // config changed, update config & edit buffer
        let config = config.unwrap();
        info!("Installing config {:?}", &config.name);

        state.config.replace(config);
    }

    let quirks = state.config.map(|c| c.midi_quirks)
        .unwrap_or_else(|| MidiQuirks::empty());
    midi_in_out_start(state, midi_in, midi_out, midi_channel, quirks, config_changed);

    config_changed
}

fn wire_ui_controls(
    controller: Arc<Mutex<Controller>>, objs: &ObjectList, callbacks: &mut Callbacks,
    app_event_tx: broadcast::Sender<AppEvent>
) -> Result<()> {
    wire(controller.clone(), objs, callbacks)?;

    // set defaults
    controller.set("program", Program::ManualMode.into(), StoreOrigin::NONE);
    controller.set("program:prev", Program::ManualMode.into(), StoreOrigin::NONE);

    let load_patch_button =
        objs.obj_by_name("load_patch_button").ok().unwrap()
            .dynamic_cast::<gtk::Button>().unwrap();
    let store_patch_button =
        objs.obj_by_name("store_patch_button").ok().unwrap()
            .dynamic_cast::<gtk::Button>().unwrap();

    Templated::init_template(&load_patch_button);
    Templated::init_template(&store_patch_button);

    let mut builder = LogicBuilder::new(controller.clone(), objs.clone(), callbacks);
    builder
        .data((load_patch_button, store_patch_button))
        .on("program")
        .run(move |v, _, _, (load_patch_button,store_patch_button)| {
            let s = program_id_string(v as usize);
            let h = hashmap! { "program" => s.as_str() };
            load_patch_button.render_template(&h);
            store_patch_button.render_template(&h);
            if v > 900 {
                // This is a hacky way of blanket-selecting all the special
                // buttons. The real HACK is that we know that the patch
                // buttons are disabled in this case and we change the tooltip
                let t = "Not available in this mode";
                load_patch_button.set_tooltip_text(Some(t));
                store_patch_button.set_tooltip_text(Some(t));
            }
        });

    let mut builder = LogicBuilder::new(controller, objs.clone(), callbacks);
    builder
        .data(app_event_tx.clone())
        .on("program")
        .run(move |v, _, origin, app_event_tx| {
            let origin = match Origin::try_from(origin) {
                Ok(v) => v,
                Err(err) => {
                    error!("Program signal handler: {}", err);
                    return;
                }
            };
            if v >= 1000 { return } // hidden program button, no PC events
            let e = ProgramChangeEvent { program: v.into(), origin };
            app_event_tx.send_or_warn(AppEvent::ProgramChange(e));
        })
        .on("load_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferLoadEvent { buffer: Buffer::EditBuffer, origin: Origin::UI };
            app_event_tx.send_or_warn(AppEvent::Load(e));
        })
        .on("load_patch_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferLoadEvent { buffer: Buffer::Current, origin: Origin::UI };
            app_event_tx.send_or_warn(AppEvent::Load(e));
        })
        .on("load_all_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferLoadEvent { buffer: Buffer::All, origin: Origin::UI };
            app_event_tx.send_or_warn(AppEvent::Load(e));
        })
        .on("store_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferStoreEvent { buffer: Buffer::EditBuffer, origin: Origin::UI };
            app_event_tx.send_or_warn(AppEvent::Store(e));
        })
        .on("store_patch_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferStoreEvent { buffer: Buffer::Current, origin: Origin::UI };
            app_event_tx.send_or_warn(AppEvent::Store(e));
        })
        .on("store_all_button")
        .run(move |_,_,_,app_event_tx| {
            let e = BufferStoreEvent { buffer: Buffer::All, origin: Origin::UI };
            app_event_tx.send_or_warn(AppEvent::Store(e));
        });

    Ok(())
}

fn wire_open_button(ui: &gtk::Builder, window: &gtk::Window) {
    let open_button = ui.object::<gtk::ToggleButton>("open_button").unwrap();
    open_button.connect_clicked({
        let window = window.clone();
        let grid = ui.object::<gtk::Grid>("program_grid").unwrap();
        move |button| {
            let is_active = button.is_active();
            // dynamically look up the current ProgramGrid widget from the UI
            ObjectList::from_widget(&grid)
                .objects_by_type::<ProgramGrid>().next()
                .map(|g| {
                    grid.remove(g);
                    let w = if is_active { g.num_pages() * 2 } else { 2 };
                    grid.attach(g, 0, 1, w as i32, 18);
                    g.set_open(is_active);
                });
            if !button.is_active() {
                make_window_smaller(window.clone());
            }
        }
    });
}

async fn controller_rx_handler<F>(rx: &mut broadcast::Receiver<Event<String,u16>>,
                                  objs: &ObjectList, callbacks: &Callbacks,
                                  f: F) -> bool
    where F: Fn(String, u16, StoreOrigin) -> ()
{
    let (name, value, origin) = match rx.recv().await {
        Ok(Event { key, value, origin, .. }) => { (key, value, origin) }
        Err(RecvError::Closed) => { return true; }
        Err(RecvError::Lagged(_)) => { return false; }
    };

    let vec = callbacks.get_vec(&name);
    match vec {
        None => { warn!("No UI callback for '{}'", &name); },
        Some(vec) => for cb in vec {
            cb()
        }
    }
    animate(&objs, &name, value);

    f(name, value, origin);
    false
}
/*
async fn controller_rx_handler_nop(rx: &mut broadcast::Receiver<Event<String>>,
                                   controller: &Arc<Mutex<Controller>>,
                                   objs: &ObjectList, callbacks: &Callbacks) -> bool {
    controller_rx_handler(rx, controller, objs, callbacks, |_,_,_| {}).await
}
*/
fn start_controller_rx<F>(controller: Arc<Mutex<Controller>>,
                          objs: ObjectList, callbacks: Callbacks,
                          f: F)
    where F: Fn(String, u16, StoreOrigin) -> () + 'static
{
    let (tx, mut rx) = broadcast::channel::<Event<String,u16>>(MIDI_OUT_CHANNEL_CAPACITY);
    controller.broadcast(Some(tx));

    glib::MainContext::default()
        .spawn_local_with_priority(
            glib::PRIORITY_HIGH,
            async move {
                let id = next_thread_id();
                info!("Controller RX thread {:?} start", id);
                loop {
                    let stop = controller_rx_handler(&mut rx, &objs, &callbacks, &f).await;
                    if stop { break; }
                }
                info!("Controller RX thread {:?} stop", id);
            });
}

async fn names_rx_handler(rx: &mut broadcast::Receiver<Event<usize,String>>,
                          ui_tx: &glib::Sender<UIEvent>) -> bool
{
    let (idx, name) = match rx.recv().await {
        Ok(Event { key, value, .. }) => { (key, value) }
        Err(RecvError::Closed) => { return true; }
        Err(RecvError::Lagged(_)) => { return false; }
    };

    ui_tx.send(UIEvent::Name(idx, name)).unwrap();

    false
}

fn start_names_rx(ui_tx: glib::Sender<UIEvent>,
                  names: Arc<Mutex<ProgramsDump>>)
{
    let (tx, mut rx) = broadcast::channel::<Event<usize,String>>(MIDI_OUT_CHANNEL_CAPACITY);
    names.lock().unwrap().broadcast_names(Some(tx));

    tokio::spawn(async move {
        let id = next_thread_id();
        info!("Program names RX thread {:?} start", id);
        loop {
            let stop = names_rx_handler(&mut rx, &ui_tx).await;
            if stop { break; }
        }
        info!("Program names RX thread {:?} stop", id);

    });
}


/// Try very hard to convince a GTK window to resize to something smaller.
/// It is not enough to do `window.resize(1, 1)` once, you have to do it
/// at the right time, so we'll try for 2 seconds to do that while also
/// tracking the window's allocation to see if it actually got smaller...
fn make_window_smaller(window: gtk::Window) {
    let start = Instant::now();
    let mut allocation = Rc::new(window.allocation());
    let mut already_smaller = Rc::new(false);

    glib::timeout_add_local(
        Duration::from_millis(100),
        move || {
            let elapsed = start.elapsed().as_millis();
            let mut cont = if elapsed > 2000 { false } else { true };

            let now = window.allocation();
            //println!("{:?} -> {:?}", allocation, now);

            let w_smaller = now.width() < allocation.width();
            let h_smaller = now.height() < allocation.height();
            let smaller = w_smaller || h_smaller;

            let w_same = now.width() == allocation.width();
            let h_same = now.height() == allocation.height();
            let same = w_same || h_same;

            if same && *already_smaller {
                // we're done
                cont = false;
            } else {
                // record progress and try again
                Rc::get_mut(&mut allocation).map(|v| *v = now);
                Rc::get_mut(&mut already_smaller).map(|v| *v = smaller);
                window.resize(1, 1);
            }

            Continue(cont)
        });
}

pub fn ui_modified_handler(ctx: &Ctx, event: &ModifiedEvent, ui_event_tx: &glib::Sender<UIEvent>) {
    match event.buffer {
        Buffer::EditBuffer => { /* don't touch event buffer */ }
        Buffer::Current => {
            let program = match ctx.program() {
                Program::Program(v) => { v as usize }
                _ => { return; }
            };
            ui_event_tx.send(UIEvent::Modified(program, event.modified))
                .unwrap();
        }
        Buffer::Program(program) => {
            ui_event_tx.send(UIEvent::Modified(program, event.modified))
                .unwrap();
        }
        Buffer::All => {
            for program in 0 .. ctx.config.program_num {
                ui_event_tx.send(UIEvent::Modified(program, event.modified))
                    .unwrap();
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = sentry::init((option_env!("SENTRY_DSN"), sentry::ClientOptions {
        release: Some(VERSION.as_str().into()),
        ..Default::default()
    }));
    let sentry_enabled = _guard.is_enabled();
    simple_logger::SimpleLogger::new()
        .with_level(LevelFilter::Trace)
        .env()
        .init()?;

    let title = format!("POD UI {}", &*VERSION);
    info!("Starting {} ({})", &title, &current_platform());

    register_module(pod_mod_pod2::module())?;
    register_module(pod_mod_pocket::module())?;
    register_module(pod_mod_xt::module())?;
    register_module(pod_mod_bass::module())?;

    let help_text = generate_help_text()?;
    let cli = Command::new("Pod UI")
        .version(VERSION.as_str())
        .after_help(&*help_text)
        .after_long_help(&*help_text);

    let cli = Opts::augment_args(cli);
    let opts: Opts = Opts::from_arg_matches(&cli.get_matches())?;
    drop(help_text);

    let (app_event_tx, mut app_event_rx) = broadcast::channel::<AppEvent>(MIDI_OUT_CHANNEL_CAPACITY);
    let (ui_event_tx, ui_event_rx) = glib::MainContext::channel::<UIEvent>(glib::PRIORITY_DEFAULT);
    let state = Arc::new(Mutex::new(State {
        midi_in_name: None,
        midi_in_cancel: None,
        midi_in_handle: None,
        midi_out_name: None,
        midi_out_cancel: None,
        midi_out_handle: None,
        midi_channel_num: 0,
        app_event_tx: app_event_tx.clone(),
        ui_event_tx: ui_event_tx.clone(),
        config: None,
        detected: None,
    }));
    let ctx_share = Arc::new(Mutex::new(Option::<Ctx>::None));

    // UI

    // glib::set_program_name needs to come before gtk::init!
    glib::set_program_name(Some(&title));

    gtk::init()
        .with_context(|| "Failed to initialize GTK")?;

    if let Some(path) = env::var("GTK_ADD_ICON_PATH").ok() {
        let icon_theme = gtk::IconTheme::default().unwrap();
        for path in path.split(":") {
            debug!("Adding icon path: {}", path);
            icon_theme.append_search_path(std::path::Path::new(path));
        }
    }

    let ui = gtk::Builder::from_string(include_str!("ui.glade"));
    let ui_objects = ObjectList::new(&ui);
    let mut ui_callbacks = Callbacks::new();
    let ui_controller = Arc::new(Mutex::new(Controller::new((*UI_CONTROLS).clone())));

    let window: gtk::Window = ui.object("ui_win").unwrap();
    window.set_title(&title);
    window.connect_delete_event({
        let app_event_tx = app_event_tx.clone();
        move |_, _| {
            info!("Shutting down...");
            app_event_tx.send_or_warn(AppEvent::Shutdown);
            Inhibit(true)
        }
    });

    set_app_icon(&window)?;

    // Re-parent window content into a notification overlay
    let widget = window.child().unwrap();
    window.remove(&widget);
    let overlay = NotificationOverlay::new();
    window.add(&overlay);
    overlay.add(&widget);

    wire_ui_controls(ui_controller.clone(), &ui_objects, &mut ui_callbacks,
                     app_event_tx.clone())?;
    wire_settings_dialog(state.clone(), &ui, &window);
    wire_panic_indicator(state.clone());
    wire_open_button(&ui, &window);

    let css = gtk::CssProvider::new();
    css.load_from_data(include_str!("default.css").as_bytes())
        .unwrap_or_else(|e| error!("Failed to load default CSS: {}", e.message()));
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().expect("Error initializing GTK CSS provider"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION
    );

    // autodetect or open devices specified on command line
    autodetect::detect(state.clone(), opts, &window)?;
    new_release_check(&app_event_tx);

    // app event handling in a separate thread
    tokio::spawn({
        let app_event_tx = app_event_tx.clone();
        let ui_event_tx = ui_event_tx.clone();
        let ctx_share = ctx_share.clone();

        async move {
            let mut ctx: Option<Ctx> = None;

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
                debug!("== {:?}", msg);

                // execute device-specific handlers
                if let Some(ctx) = &ctx {
                    match &msg {
                        // device inquiry
                        AppEvent::MidiMsgIn(msg @ MidiMessage::UniversalDeviceInquiry { .. }) |
                        AppEvent::MidiMsgIn(msg @ MidiMessage::UniversalDeviceInquiryResponse { .. }) => {
                            midi_udi_handler(ctx, msg);
                        }

                        // control change
                        AppEvent::MidiMsgIn(msg @ MidiMessage::ControlChange { .. }) => {
                            midi_cc_in_handler(ctx, msg);
                        }
                        AppEvent::MidiMsgOut(msg @ MidiMessage::ControlChange { .. }) => {
                            midi_cc_out_handler(ctx, msg);
                        }
                        AppEvent::ControlChange(cc) => {
                            cc_handler(ctx, cc);
                        }

                        // program change
                        AppEvent::MidiMsgIn(msg @ MidiMessage::ProgramChange { .. }) => {
                            midi_pc_in_handler(ctx, msg);
                        }
                        AppEvent::MidiMsgOut(msg @ MidiMessage::ProgramChange { .. }) => {
                            midi_pc_out_handler(ctx, msg);
                        }
                        AppEvent::ProgramChange(pc) => {
                            pc_handler(ctx, pc);
                        }

                        // store & load
                        AppEvent::Load(event) => {
                            load_handler(ctx, event)
                        }
                        AppEvent::Store(event) => {
                            store_handler(ctx, event)
                        }
                        AppEvent::Copy(event) => {
                            copy_handler(ctx, event)
                        }
                        AppEvent::BufferData(event) => {
                            buffer_handler(ctx, event)
                        }
                        AppEvent::Modified(event) => {
                            modified_handler(ctx, event);
                            ui_modified_handler(ctx, event, &ui_event_tx)
                        }

                        // other
                        AppEvent::MidiMsgIn(msg) => {
                            midi_in_handler(ctx, msg);
                        }
                        AppEvent::MidiMsgOut(msg) => {
                            midi_out_handler(ctx, msg);
                        }
                        AppEvent::Marker(marker) => {
                           marker_handler(ctx, *marker);
                        }

                        // silently ignoring
                        AppEvent::MidiIn(_) | AppEvent::MidiOut(_)  => { /* handled in MIDI OUT thread */ }
                        e if is_system_app_event(e) => {}

                        // error message
                        _ => {
                            error!("Unhandled app event: {:?}", msg);
                        }
                    }
                } else {
                    if !is_system_app_event(&msg) {
                        warn!("MIDI CC event {:?} without context", msg);
                    }
                }

                // execute system handlers
                match &msg {
                    // device detected
                    AppEvent::DeviceDetected(event) => {
                        sentry_set_device_tags(
                            &event.name,
                            &event.version,
                            &ctx.as_ref().map(|ctx| ctx.config.name.clone())
                                .unwrap_or_else(|| "?".into())
                        );
                        ui_event_tx.send_or_warn(UIEvent::DeviceDetected(event.clone()));
                    }
                    // forward notification messages to the UI thread
                    AppEvent::Notification(event) => {
                        ui_event_tx.send_or_warn(UIEvent::Notification(event.msg.clone(), event.id.clone()));
                    }
                    // new config & shutdown
                    AppEvent::NewConfig(event) => {
                        if event.midi_changed {
                            if let Some(ctx) = &ctx {
                                ctx.set_midi_channel(event.midi_channel);
                            }
                            ui_event_tx.send_or_warn(UIEvent::NewMidiConnection);
                        }
                        if event.config_changed {
                            // transfer Ctx ownership to the UI thread and
                            // ask it to initialize a new Ctx
                            let mut ctx_share = ctx_share.lock().unwrap();
                            *ctx_share = ctx.take();

                            ui_event_tx.send_or_warn(UIEvent::NewConfig);
                        }
                    }
                    AppEvent::NewCtx => {
                        trace!("New context installed...");
                        let mut ctx_share = ctx_share.lock().unwrap();
                        ctx.replace(ctx_share.take().unwrap());
                        new_device_handler(ctx.as_ref().unwrap());
                    }
                    AppEvent::Shutdown => {
                        ui_event_tx.send_or_warn(UIEvent::Shutdown);
                    }

                    // message conversion
                    AppEvent::MidiIn(bytes) => {
                        // todo: do not clone
                        // todo: report to sentry if message conversion failed?
                        if let Some(msg) = MidiMessage::from_bytes(bytes.clone())
                            .map_err(|e| error!("{}", e)).ok() {
                            app_event_tx.send_or_warn(AppEvent::MidiMsgIn(msg));
                        }
                    }
                    AppEvent::MidiMsgOut(msg) => {
                        let bytes = MidiMessage::to_bytes(&msg);
                        app_event_tx.send_or_warn(AppEvent::MidiOut(bytes));
                    }

                    // silently ignore everything else
                    _ => {}
                }
            }

        }
    });

    // run UI controller callback on the GTK thread
    start_controller_rx(ui_controller.clone(), ui_objects, ui_callbacks, |_,_,_| {});

    // run UI event handling on the GTK thread
    ui_event_rx.attach(None, {
        let ui_event_tx = ui_event_tx.clone();

        let mut program_grid: Option<ProgramGrid> = None;
        let mut shutting_down = false;
        let window = window.clone();
        let header_bar: gtk::HeaderBar = ui.object("header_bar").unwrap();

        let transfer_icon_up: gtk::Label = ui.object("transfer_icon_up").unwrap();
        let transfer_icon_down: gtk::Label = ui.object("transfer_icon_down").unwrap();
        transfer_icon_up.set_opacity(0.0);
        transfer_icon_down.set_opacity(0.0);

        let transfer_up_sem = Arc::new(atomic::AtomicI32::new(0));
        let transfer_down_sem = Arc::new(atomic::AtomicI32::new(0));

        move |event| {
            match event {
                UIEvent::NewConfig => {
                    dispatch_buffer_clear();

                    let state = state.lock().unwrap();
                    let config = state.config.unwrap();

                    info!("Initiating module for config {:?}", &config.name);
                    let interface = init_module(config)
                        .map_err(|err| error!("Failed to initialize config {:?}: {}", config.name, err))
                        .ok();
                    let Some(interface) = interface else {
                        // Failed to initialize the interface, so skip the rest
                        return Continue(true);
                    };

                    if let Some(ctx) = &*ctx_share.lock().unwrap() {
                        // close channels
                        ctx.controller.broadcast(None);
                        ctx.dump.lock().unwrap().broadcast_names(None);
                    }

                    let handler = interface.handler;
                    let controller = interface.edit_buffer.lock().unwrap().controller();
                    let objs = interface.objects;
                    let callbacks = interface.callbacks;

                    {
                        // start event handlers
                        let app_event_tx = app_event_tx.clone();
                        start_controller_rx(
                            controller.clone(), objs, callbacks,
                            move |name, value, origin| {
                                let e = ControlChangeEvent { name, value, origin };
                                app_event_tx.send_or_warn(AppEvent::ControlChange(e));
                            }
                        );

                        start_names_rx(ui_event_tx.clone(), interface.dump.clone());
                    }

                    let ctx = Ctx {
                        config,
                        controller,
                        handler,
                        edit: interface.edit_buffer.clone(),
                        dump: interface.dump.clone(),
                        ui_controller: ui_controller.clone(),
                        app_event_tx: app_event_tx.clone()
                    };
                    ctx_share.lock().unwrap().replace(ctx);
                    app_event_tx.send_or_warn(AppEvent::NewCtx);

                    // attach new device UI

                    let device_box: gtk::Box = ui.object("device_box").unwrap();
                    device_box.foreach(|w| device_box.remove(w));
                    device_box.add(&interface.widget);

                    // Another ugly hack: to initialize the UI, it is not enough
                    // to animate() the init_controls, since the controls do
                    // emit other (synthetic or otherwise) animate() calls in their
                    // wiring. So, we both animate() as part of init_module()
                    // (needed to hide most controls that hide before first show)
                    // and defer an init_module_controls() call that needs to happen
                    // after the rx is subscribed to again!
                    glib::idle_add_local_once({
                        let config = config.clone();
                        let edit_buffer = interface.edit_buffer.clone();

                        move || {
                            let edit_buffer = edit_buffer.lock().unwrap();
                            init_module_controls(&config, &edit_buffer)
                                .unwrap_or_else(|err| error!("{}", err));
                        }
                    });

                    // Update UI from state after device change
                    //update_ui_from_state(&state, &mut ui_controller.lock().unwrap());

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

                    let program_num = config.program_num;
                    let g = ProgramGrid::new(program_num);
                    grid.attach(&g, 0, 1, 2, 18);
                    g.connect_action({
                        let app_event_tx = app_event_tx.clone();
                        move |action| {
                            match action {
                                ProgramGridAction::Load { program } => {
                                    let e = BufferCopyEvent {
                                        from: Buffer::Program(program),
                                        to: Buffer::EditBuffer
                                    };
                                    app_event_tx.send_or_warn(AppEvent::Copy(e));
                                }
                                ProgramGridAction::LoadUnmodified { program } => {
                                    dispatch_buffer_set(Buffer::Program(program), Buffer::EditBuffer);
                                    let e = BufferLoadEvent { buffer: Buffer::Program(program), origin: Origin::UI };
                                    app_event_tx.send_or_warn(AppEvent::Load(e));
                                }
                                ProgramGridAction::Store { program } => {
                                    let e = BufferCopyEvent {
                                        from: Buffer::EditBuffer,
                                        to: Buffer::Program(program),
                                    };
                                    app_event_tx.send_or_warn(AppEvent::Copy(e));
                                }
                                ProgramGridAction::LoadDevice { program } => {
                                    let e = BufferLoadEvent { buffer: Buffer::Program(program), origin: Origin::UI };
                                    app_event_tx.send_or_warn(AppEvent::Load(e));
                                }
                                ProgramGridAction::StoreDevice { program } => {
                                    let e = BufferStoreEvent { buffer: Buffer::Program(program), origin: Origin::UI };
                                    app_event_tx.send_or_warn(AppEvent::Store(e));
                                }
                            };
                        }
                    });
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

                    program_grid.replace(g);

                    // update ui from config
                    ui_controller.set_full("manual_mode_present",
                                           state.config.unwrap().flags.contains(DeviceFlags::MANUAL_MODE) as u16,
                                           StoreOrigin::NONE, Signal::Force);
                    ui_controller.set_full("tuner_present",
                                           1, StoreOrigin::NONE, Signal::Force);
                    // activate the hidden "program:1000" button, deactivating
                    // all other program buttons
                    ui_controller.set_full("program",
                                           1000, StoreOrigin::NONE, Signal::Force);

                    make_window_smaller(window.clone());
                }
                UIEvent::Modified(page, modified) => {
                    if let Some(grid) = &program_grid {
                        grid.set_program_modified(page, modified);
                    }
                }
                UIEvent::Name(page, name) => {
                    if let Some(grid) = &program_grid {
                        grid.set_program_name(page, &name);
                    }
                }
                UIEvent::MidiTx => {
                    transfer_icon_up.set_opacity(1.0);
                    transfer_up_sem.fetch_add(1, atomic::Ordering::SeqCst);
                    {
                        let transfer_icon_up = transfer_icon_up.clone();
                        let transfer_up_sem = Arc::clone(&transfer_up_sem);
                        glib::timeout_add_local_once(
                            Duration::from_millis(500),
                            move || {
                                let v = transfer_up_sem.fetch_add(-1, atomic::Ordering::SeqCst);
                                if v <= 1 {
                                    transfer_icon_up.set_opacity(0.0);
                                }
                            });
                    }
                }
                UIEvent::MidiRx => {
                    transfer_icon_down.set_opacity(1.0);
                    transfer_down_sem.fetch_add(1, atomic::Ordering::SeqCst);
                    {
                        let transfer_icon_down = transfer_icon_down.clone();
                        let transfer_down_sem = Arc::clone(&transfer_down_sem);
                        glib::timeout_add_local_once(
                            Duration::from_millis(500),
                            move || {
                                let v = transfer_down_sem.fetch_add(-1, atomic::Ordering::SeqCst);
                                if v <= 1 {
                                    transfer_icon_down.set_opacity(0.0);
                                }
                            });
                    }
                }
                UIEvent::DeviceDetected(event) => {
                    // TODO: this, strictly speaking, doesn't need to be in State,
                    //       it can be a local to the UI thread
                    let mut state = state.lock().unwrap();
                    state.detected.replace(event);
                    ui_event_tx.send_or_warn(UIEvent::NewMidiConnection);
                }
                UIEvent::NewMidiConnection => {
                    let state = state.lock().unwrap();
                    let midi_in_name = state.midi_in_name.as_ref();
                    let midi_out_name = state.midi_out_name.as_ref();
                    let name = {
                        let config_name = &state.config.map(|c| c.name.clone())
                            .unwrap_or_else(|| String::new());
                        let (detected_name, detected_ver) = state.detected
                            .as_ref()
                            .map(|d| (d.name.clone(), d.version.clone()))
                            .unwrap_or_else(|| (String::new(), String::new()));

                        match (&detected_name, config_name) {
                            (_, b) if b.is_empty() => {
                                String::new()
                            }
                            (a, b) if a.is_empty() => {
                                b.clone()
                            },
                            (a, b) if a == b => {
                                format!("{} {}", detected_name, detected_ver)
                            },
                            _ => {
                                format!("{} {} as {}", detected_name, detected_ver, config_name)
                            }
                        }
                    };
                    let subtitle = match (midi_in_name, midi_out_name) {
                        (None, _) | (_, None) => {
                            "no device connected".to_string()
                        }
                        (Some(a), Some(b)) if a == b => {
                            format!("{}", a)
                        }
                        (Some(a), Some(b)) => {
                            format!("i: {} / o: {}", a, b)
                        }
                    };
                    let subtitle = if name.is_empty() {
                        subtitle
                    } else {
                        format!("{} @ {}", name, subtitle)
                    };

                    header_bar.set_subtitle(Some(&subtitle));
                }
                UIEvent::Notification(msg, id) => {
                    if let Some(id) = id.as_ref() {
                        let updated = overlay.update_notification_with_id(msg.as_str(), id);
                        if !updated {
                            overlay.add_notification_with_id(msg.as_str(), id);
                        }
                    } else {
                        overlay.add_notification(msg.as_str());
                    }
                }
                UIEvent::Shutdown if !shutting_down => {
                    header_bar.set_subtitle(Some("Shutting down..."));
                    shutting_down = true;

                    let mut state = state.lock().unwrap();
                    let handle = midi_in_out_stop(&mut state);
                    let ui_tx = ui_event_tx.clone();
                    tokio::spawn(async move {
                        handle.await;
                        ui_tx.send(UIEvent::Quit).unwrap_or_default();
                    });
                }
                UIEvent::Panic => {
                    let tooltip = if sentry_enabled {
                        Some("\
                                Something broke in the app and one of its internal \
                                processing threads crashed. You can check the logs to see what \
                                exactly happened. The error has been reported to the cloud.\
                                ")
                    } else { None };
                    if let Some(widget) = ui.object::<gtk::Widget>("panic_indicator") {
                        widget.set_visible(true);
                        if tooltip.is_some() {
                            widget.set_tooltip_text(tooltip);
                        }
                    }
                }
                UIEvent::Shutdown => {
                    // for the impatient ones that press the "close" button
                    // again while shut down is in progress...
                    header_bar.set_subtitle(Some("SHUTTING DOWN. PLEASE WAIT..."));
                }
                UIEvent::Quit => {
                    info!("Quitting...");
                    // application is being closed, perform clean-up
                    // that needs to happen inside the GTK thread...

                    // detach the program buttons so that there are no
                    // "GTK signal handler not found" errors when SignalHandler
                    // objects are dropped...
                    let r = ui.object::<gtk::RadioButton>("program").unwrap();
                    if let Some(g) = &program_grid {
                        g.join_radio_group(Option::<&gtk::RadioButton>::None);
                    }
                    r.emit_by_name::<()>("group-changed", &[]);

                    // quit
                    gtk::main_quit();
                }
            }


            Continue(true)
        }
    });

    // crash testing
    if let Some(duration) = std::env::var_os("PODUI_CRASH_TEST")
        .and_then(|str| str.into_string().ok())
        .and_then(|str| str.parse::<u64>().ok()) {
        info!("THREAD CRASH TESTING");
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(duration)).await;
            let _ = 1 / 0;
        });
    }

    // show the window and do init stuff...
    window.show_all();
    window.resize(1, 1);

    debug!("starting gtk main loop");
    gtk::main();
    debug!("end of gtk main loop");

    Ok(())
}