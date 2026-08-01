//! Builds a device panel `gtk::Widget` tree by populating a base `.glade`
//! layout from a HOCON layout description.
//!
//! The base glade file supplies the static chrome that doesn't vary with
//! content - the top-level `app_win` window, the "Program" name entry/frame -
//! plus two containers this module fills in:
//! - `toggles`: a `GtkGrid`, one checkbox attached per top-level toggle
//! - `controls`: a `GtkBox`, one column box packed per distinct `column`
//!   value among the parsed groups
//!
//! The `.conf` file only describes *content*: which groups exist, which
//! control each widget binds to, and any conditional visibility/enable
//! rules. Container mechanics for the dynamic parts (Box vs Grid, spacing,
//! margins, orientation) are fixed by the Rust code for each node kind
//! below, not authored in the `.conf` file.
//!
//! The resulting widget tree uses exactly the same conventions the existing
//! generic wiring engine already expects, so `wire()`, `wire_frames()`,
//! `wire_toggles()` and `animate()` (see `wiring.rs`/`util.rs`/`toggles.rs`)
//! work against it completely unchanged:
//! - a widget's GTK `name` property is the control key it binds to
//! - radio buttons in a group are named `"{control}:{value}"`
//! - `rules` are passed through verbatim as CSS classes, in the exact
//!   `"{control}={value}:{action}"` / `"{control}=*:{action}"` grammar
//!   `animate()` already parses (action ∈ show/hide/enable/disable/opacity=0/opacity=1)
//! - a group with a `toggle` control gets a `"toggle:{control}"` class

use std::fs;
use std::path::Path;

use anyhow::*;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;

use crate::prelude::*;

/// Horizontal margin applied around a group frame's control grid.
const CONTENT_MARGIN_H: i32 = 12;
/// Margin left below a group frame's control grid, before the frame's own edge.
const CONTENT_MARGIN_BOTTOM: i32 = 4;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct LayoutDef {
    /// Base Glade layout file path relative to the config file
    ui: String,
    #[serde(default)]
    toggles: Vec<ToggleDef>,
    #[serde(default)]
    groups: Vec<GroupDef>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct ToggleDef {
    control: String,
    label: String,
    #[serde(default)]
    rules: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct GroupDef {
    group: String,
    column: u32,
    #[serde(default)]
    toggle: Option<String>,
    #[serde(default)]
    rules: Vec<String>,
    #[serde(default)]
    controls: Vec<ControlDef>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct ControlDef {
    control: String,
    widget: WidgetKind,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    options: Vec<RadioOption>,
    /// CSS classes applied to the control widget and label.
    #[serde(default)]
    rules: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct RadioOption {
    value: u16,
    label: String,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum WidgetKind {
    Slider,
    Toggle,
    Combo,
    RadioGroup,
    Entry,
    Button,
}

// ---

struct SizeControl {
    /// all widgets vertical size group
    v_group: gtk::SizeGroup,
    /// labels horizontal size group
    h_group_labels: gtk::SizeGroup,
}

impl SizeControl {
    pub fn new() -> Self {
        SizeControl {
            v_group: gtk::SizeGroup::new(gtk::SizeGroupMode::Vertical),
            h_group_labels: gtk::SizeGroup::new(gtk::SizeGroupMode::Horizontal),
        }
    }

    pub fn add_widget<T: IsA<gtk::Widget>>(&self, widget: &T) {
        self.v_group.add_widget(widget)
    }

    pub fn add_label<T: IsA<gtk::Widget>>(&self, widget: &T) {
        self.v_group.add_widget(widget);
        self.h_group_labels.add_widget(widget);
    }
}

// ---

/// Read and parse `conf_path`, load the base Glade layout file
/// (relative to the config file path, following resource resolution).
/// Returns the resulting `gtk::Widget` populated with the layout
/// and UI constructed from the layout definition.
pub fn build_ui(conf_path: &str) -> Result<gtk::Widget> {

    let real_conf_path = pod_core::resources::resolve(conf_path)
        .map_err(|e| anyhow!("Failed to resolve {:?}: {}", conf_path, e))?;
    let conf_str = fs::read_to_string(&real_conf_path)
        .with_context(|| format!("Failed to read conf file {:?}", conf_path))?;
    let layout: LayoutDef = hocon::de::from_str(&conf_str)
        .map_err(|e| anyhow!("Failed to parse layout {:?}: {}", conf_path, e))?;

    let glade_path = Path::new(conf_path).parent().unwrap_or_else(|| Path::new("."))
        .join(&layout.ui);
    let real_glade_path = pod_core::resources::resolve(&glade_path.to_string_lossy())
        .map_err(|e| anyhow!("Failed to resolve {:?}: {}", glade_path, e))?;
    let base_glade = fs::read_to_string(&real_glade_path)
        .with_context(|| format!("Failed to read glade file {:?}", glade_path))?;

    let builder = gtk::Builder::from_string(&base_glade);

    let window: gtk::Window = builder.object("app_win")
        .context("base glade layout is missing an 'app_win' window")?;
    let root = window.child()
        .context("base glade layout's 'app_win' window has no child")?;
    window.remove(&root);

    let size_control = SizeControl::new();

    let toggles_grid: gtk::Grid = builder.object("toggles")
        .context("base glade layout is missing a 'toggles' grid")?;
    populate_toggle_bar(&toggles_grid, &layout.toggles, &size_control);

    let controls_box: gtk::Box = builder.object("controls")
        .context("base glade layout is missing a 'controls' box")?;
    populate_columns(&controls_box, &layout.groups, &size_control);

    root.show_all();
    Ok(root)
}

fn populate_toggle_bar(grid: &gtk::Grid, toggles: &[ToggleDef], size_control: &SizeControl) {
    for (i, t) in toggles.iter().enumerate() {
        let check = gtk::CheckButton::with_label(t.label.as_str());
        check.set_widget_name(&t.control);
        apply_rules(&check, &t.rules);
        size_control.add_widget(&check);
        grid.attach(&check, i as i32, 1, 1, 1);
    }
}

fn populate_columns(controls_box: &gtk::Box, groups: &[GroupDef], size_control: &SizeControl) {
    let max_column = groups.iter().map(|g| g.column).max().unwrap_or(0);
    for column in 0..=max_column {
        let col_box = gtk::Box::new(gtk::Orientation::Vertical, 5);
        for g in groups.iter().filter(|g| g.column == column) {
            col_box.pack_start(&build_group(g, size_control), false, true, 0);
        }
        controls_box.pack_start(&col_box, true, true, 0);
    }
}

fn build_group(g: &GroupDef, size_control: &SizeControl) -> gtk::Widget {
    let frame = gtk::Frame::new(Some(g.group.as_str()));
    frame.set_label_align(0.0, 0.5);
    frame.set_shadow_type(gtk::ShadowType::EtchedOut);

    if let Some(toggle) = &g.toggle {
        frame.style_context().add_class(&format!("toggle:{}", toggle));
    }
    apply_rules(&frame, &g.rules);

    let grid = gtk::Grid::new();
    grid.set_column_spacing(10);
    apply_content_margins(&grid);

    for (row, c) in g.controls.iter().enumerate() {
        build_control_row(&grid, row as i32, c, size_control);
    }

    frame.add(&grid);
    frame.upcast()
}

fn build_control_row(grid: &gtk::Grid, row: i32, c: &ControlDef, size_control: &SizeControl) {
    let widget = build_control_widget(c, size_control);
    widget.set_hexpand(true);
    apply_rules(&widget, &c.rules);

    match &c.label {
        Some(label) => {
            let l = gtk::Label::new(Some(label.as_str()));
            // original glade layout has "horizontal size group + halign = end",
            // but here that produces centered labels. therefore, we set xalign
            // instead of haligh -- seems to work...
            l.set_xalign(1.0);

            // Implement click on label to focus the widget.
            // GtkLabel needs an event box, but when the frame is
            // set insensitive, the insensitive event box will
            // swallow mouse clicks. Place event box and
            // label into an overlay and when the label is set
            // insensitive, the event box is toggled hidden
            let event_box = gtk::EventBox::builder().visible_window(false).build();
            event_box.connect_button_press_event({
                let widget = widget.clone();
                move |_, event| {
                    if event.event_type() != gdk::EventType::ButtonPress { return Propagation::Proceed }
                    widget.grab_focus();
                    Propagation::Stop
                }
            });
            l.connect_state_flags_changed({
                let event_box = event_box.clone();
                move |l, _| {
                    event_box.set_visible(l.is_sensitive());
                }
            });

            let overlay = gtk::Overlay::new();
            overlay.add(&l);
            overlay.add_overlay(&event_box);

            apply_rules(&overlay, &c.rules);
            size_control.add_label(&overlay);

            grid.attach(&overlay, 0, row, 1, 1);
            grid.attach(&widget, 1, row, 1, 1);
        }
        None => {
            grid.attach(&widget, 0, row, 2, 1);
        }
    }
}

fn build_control_widget(c: &ControlDef, size_control: &SizeControl) -> gtk::Widget {
    match c.widget {
        WidgetKind::Slider => {
            let scale = gtk::Scale::new(gtk::Orientation::Horizontal, None::<&gtk::Adjustment>);
            scale.set_widget_name(&c.control);
            scale.set_digits(0);
            scale.set_value_pos(gtk::PositionType::Right);
            scale.upcast()
        }
        WidgetKind::Toggle => {
            // Group-level toggles are plain unlabeled checkboxes; the label
            // (if any) is a separate widget to their left, built by
            // `build_control_row` - unlike the top-level toggle bar, which
            // bakes the label into the checkbox itself (see `build_toggle_bar`).
            let check = gtk::CheckButton::new();
            check.set_widget_name(&c.control);
            size_control.add_widget(&check);
            check.upcast()
        }
        WidgetKind::Combo => {
            let combo = gtk::ComboBoxText::new();
            combo.set_widget_name(&c.control);
            size_control.add_widget(&combo);
            combo.upcast()
        }
        WidgetKind::RadioGroup => {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 5);
            let mut group: Option<gtk::RadioButton> = None;
            for opt in &c.options {
                let radio = gtk::RadioButton::with_label(opt.label.as_str());
                radio.set_widget_name(&format!("{}:{}", c.control, opt.value));
                if let Some(g) = &group {
                    radio.join_group(Some(g));
                } else {
                    group = Some(radio.clone());
                }
                radio.set_hexpand(true);
                size_control.add_widget(&radio);
                b.pack_start(&radio, false, true, 0);
            }
            // when the label calls `grab_focus`, forward the
            // focus to the radio group
            b.connect_grab_focus(move |_| {
                if let Some(group) = group.as_ref() {
                    let mut focus_set = false;
                    for radio in &group.group() {
                        if radio.is_active() {
                            focus_set = true;
                            radio.grab_focus();
                            break
                        }
                    }
                    if !focus_set {
                        group.grab_focus();
                    }
                }
            });
            b.upcast()
        }
        WidgetKind::Entry => {
            let entry = gtk::Entry::new();
            entry.set_widget_name(&c.control);
            entry.set_valign(gtk::Align::Center);
            size_control.add_widget(&entry);
            entry.upcast()
        }
        WidgetKind::Button => {
            let button = gtk::Button::with_label(c.label.as_deref().unwrap_or(""));
            button.set_widget_name(&c.control);
            size_control.add_widget(&button);
            button.upcast()
        }
    }
}

/// Matches a `"{control}={v1},{v2},...:{action}"` rule (two or more
/// comma-separated numeric values) so it can be expanded into one rule per
/// value - see `apply_rules`.
static MULTI_VALUE_RULE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^([^=]+)=(\d+(?:,\d+)+):(.+)$").unwrap()
});

fn apply_rules<T: IsA<gtk::Widget>>(widget: &T, rules: &[String]) {
    let style = widget.style_context();
    for rule in rules {
        match MULTI_VALUE_RULE.captures(rule) {
            Some(caps) => {
                let control = &caps[1];
                let action = &caps[3];
                for value in caps[2].split(',') {
                    style.add_class(&format!("{}={}:{}", control, value, action));
                }
            }
            None => style.add_class(rule),
        }
    }
}

/// Applies the standard inset (see `CONTENT_MARGIN_H`/`CONTENT_MARGIN_BOTTOM`)
/// used around a frame's content.
fn apply_content_margins<T: IsA<gtk::Widget>>(widget: &T) {
    widget.set_margin_start(CONTENT_MARGIN_H);
    widget.set_margin_end(CONTENT_MARGIN_H);
    widget.set_margin_bottom(CONTENT_MARGIN_BOTTOM);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal stand-in for a device's `pod.glade`: just the pieces `build_ui`
    // requires (`app_win`, `toggles`, `program_name`, `controls`), without any
    // of the real chrome (spacing, the "Program" frame, etc).
    const GLADE: &str = r#"
        <interface>
          <object class="GtkWindow" id="app_win">
            <child>
              <object class="GtkBox">
                <property name="visible">True</property>
                <property name="orientation">vertical</property>
                <child>
                  <object class="GtkGrid" id="toggles">
                    <property name="name">toggles</property>
                    <property name="visible">True</property>
                  </object>
                </child>
                <child>
                  <object class="GtkEntry" id="program_name">
                    <property name="name">program_name</property>
                    <property name="visible">True</property>
                  </object>
                </child>
                <child>
                  <object class="GtkBox" id="controls">
                    <property name="name">controls</property>
                    <property name="visible">True</property>
                  </object>
                </child>
              </object>
            </child>
          </object>
        </interface>
    "#;

    const CONF: &str = r#"
        ui = "pod.glade"
        toggles = [
          { control = "noise_gate_enable", label = "Gate" }
          { control = "effect_enable", label = "FX",
            rules = [ "effect_select=0:opacity=0", "effect_select=1:opacity=0", "effect_select=2:opacity=0", "effect_select=*:opacity=1" ] }
        ]
        groups = [
          { group = "Noise Gate", column = 1, toggle = "noise_gate_enable",
            rules = [ "noise_gate_enable=1:enable", "noise_gate_enable=0:disable" ],
            controls = [
              { control = "gate_threshold", widget = "slider", label = "Threshold" }
              { control = "gate_decay", widget = "slider", label = "Decay" }
            ]
          }
          { group = "Preamp", column = 1,
            controls = [
              { control = "amp_select", widget = "combo" }
              { control = "drive2", widget = "slider", label = "Drive 2",
                rules = [ "amp_select=4:show", "amp_select=*:hide" ] }
            ]
          }
          { group = "Reverb", column = 2,
            controls = [
              { control = "reverb_type", widget = "radio-group", label = "Type",
                options = [
                  { value = 0, label = "Spring" }
                  { value = 1, label = "Hall" }
                ]
              }
            ]
          }
        ]
    "#;

    #[test]
    fn parses_layout() {
        let layout: LayoutDef = hocon::de::from_str(CONF).expect("should parse");
        assert_eq!(layout.ui, "pod.glade");
        assert_eq!(layout.toggles.len(), 2);
        assert_eq!(layout.toggles[0].control, "noise_gate_enable");
        assert!(layout.toggles[0].rules.is_empty());
        assert_eq!(layout.toggles[1].rules, vec![
            "effect_select=0:opacity=0", "effect_select=1:opacity=0",
            "effect_select=2:opacity=0", "effect_select=*:opacity=1",
        ]);

        assert_eq!(layout.groups.len(), 3);
        let ng = &layout.groups[0];
        assert_eq!(ng.toggle.as_deref(), Some("noise_gate_enable"));
        assert_eq!(ng.rules, vec!["noise_gate_enable=1:enable", "noise_gate_enable=0:disable"]);

        let preamp = &layout.groups[1];
        let drive2 = &preamp.controls[1];
        assert_eq!(drive2.rules, vec!["amp_select=4:show", "amp_select=*:hide"]);

        let reverb = &layout.groups[2];
        assert_eq!(reverb.controls[0].widget, WidgetKind::RadioGroup);
        assert_eq!(reverb.controls[0].options.len(), 2);
    }

    #[test]
    fn rejects_misspelled_field() {
        const TYPO: &str = r#"
            ui = "pod.glade"
            groups = [
              { group = "Noise Gate", column = 1,
                controls = [
                  { control = "gate_threshold", widget = "slider", label = "Threshold",
                    ruels = [ "noise_gate_enable=1:enable" ] }
                ]
              }
            ]
        "#;

        let err = hocon::de::from_str::<LayoutDef>(TYPO)
            .expect_err("misspelled field name should fail to parse");
        assert!(err.to_string().contains("ruels"), "error should name the bad field: {}", err);
    }

    /// Writes `GLADE`/`CONF` out to a temp directory so `build_ui` (which
    /// now only takes a conf file path, and reads both files itself) has
    /// real files to read.
    fn write_fixture() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("pod-gtk-layout-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("should create temp dir");
        std::fs::write(dir.join("pod.glade"), GLADE).expect("should write glade fixture");
        let conf_path = dir.join("pod.conf");
        std::fs::write(&conf_path, CONF).expect("should write conf fixture");
        conf_path
    }

    #[test]
    fn builds_widget_tree() {
        gtk::init().expect("gtk::init failed - is a display available?");

        let conf_path = write_fixture();
        let widget = build_ui(&conf_path.to_string_lossy()).expect("should build");
        let objs = ObjectList::from_widget(&widget);

        // control widgets are named
        let threshold = objs.obj_by_name("gate_threshold").expect("gate_threshold widget");
        assert!(threshold.dynamic_cast_ref::<gtk::Scale>().is_some());

        // program name entry is always present
        objs.obj_by_name("program_name").expect("program_name widget");

        // toggle bar checkbox is present, named after its control
        let gate_check = objs.obj_by_name("noise_gate_enable").expect("toggle checkbox");
        assert!(gate_check.dynamic_cast_ref::<gtk::CheckButton>().is_some());

        // toggle bar rule passed through verbatim
        let fx_check = objs.obj_by_name("effect_enable").expect("fx toggle checkbox");
        let fx_classes: Vec<String> = fx_check.dynamic_cast_ref::<gtk::Widget>().unwrap()
            .style_context().list_classes().iter().map(|c| c.to_string()).collect();
        assert!(fx_classes.contains(&"effect_select=*:opacity=1".to_string()));

        // toggles grid container exists, reserved-named for wire_toggles()
        let toggles_grid = objs.obj_by_name("toggles").expect("toggles grid");
        assert!(toggles_grid.dynamic_cast_ref::<gtk::Grid>().is_some());

        // radio group members are named "{control}:{value}"
        let spring = objs.obj_by_name("reverb_type:0").expect("reverb_type:0 widget");
        let hall = objs.obj_by_name("reverb_type:1").expect("reverb_type:1 widget");
        let spring = spring.dynamic_cast_ref::<gtk::RadioButton>().unwrap();
        let hall = hall.dynamic_cast_ref::<gtk::RadioButton>().unwrap();
        assert_eq!(spring.group().len(), 2);
        assert_eq!(hall.group().len(), 2);

        // group frame carries the toggle: class (wire_frames()) and its
        // rules verbatim (animate()), in the exact existing grammar
        let frame = objs.objects_by_type::<gtk::Frame>()
            .find(|f| f.label().as_deref() == Some("Noise Gate"))
            .expect("Noise Gate frame");
        let classes: Vec<String> = frame.style_context().list_classes().iter().map(|c| c.to_string()).collect();
        assert!(classes.contains(&"toggle:noise_gate_enable".to_string()));
        assert!(classes.contains(&"noise_gate_enable=1:enable".to_string()));
        assert!(classes.contains(&"noise_gate_enable=0:disable".to_string()));

        // a control's rules pass through verbatim onto its widget
        let drive2 = objs.obj_by_name("drive2").expect("drive2 widget");
        let drive2_classes: Vec<String> = drive2.dynamic_cast_ref::<gtk::Widget>().unwrap()
            .style_context().list_classes().iter().map(|c| c.to_string()).collect();
        assert!(drive2_classes.contains(&"amp_select=4:show".to_string()));
        assert!(drive2_classes.contains(&"amp_select=*:hide".to_string()));

        // apply_rules() expands a "{control}={v1},{v2},...:{action}" rule
        // into one rule per value - checked here rather than as its own
        // #[test] since GTK tolerates only one thread touching it per
        // process, and this crate already spends that one thread on this test
        let label = gtk::Label::new(None);
        apply_rules(&label, &[
            "effect_select=0,1,2:opacity=0".to_string(),
            "effect_select=*:opacity=1".to_string(),
        ]);
        let label_classes: Vec<String> = label.style_context().list_classes()
            .iter().map(|c| c.to_string()).collect();
        assert!(label_classes.contains(&"effect_select=0:opacity=0".to_string()));
        assert!(label_classes.contains(&"effect_select=1:opacity=0".to_string()));
        assert!(label_classes.contains(&"effect_select=2:opacity=0".to_string()));
        assert!(!label_classes.iter().any(|c| c.contains(',')));
        // single-value/wildcard rules still pass through unchanged
        assert!(label_classes.contains(&"effect_select=*:opacity=1".to_string()));

        std::fs::remove_dir_all(conf_path.parent().unwrap()).ok();
    }
}
