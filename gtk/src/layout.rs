//! Builds a device panel `gtk::Widget` tree from a HOCON layout description,
//! replacing what used to be a per-device `.glade` file.
//!
//! The DSL only describes *content*: which groups exist, which control each
//! widget binds to, and any conditional visibility/enable rules. Container
//! mechanics (Box vs Grid, spacing, margins, orientation) are fixed by the
//! Rust code for each node kind below, not authored in the `.conf` file.
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

use anyhow::*;
use serde::Deserialize;

use crate::prelude::*;

/// Horizontal margin applied around a frame's content (the program-name
/// entry, each group's control grid).
const CONTENT_MARGIN_H: i32 = 12;
/// Margin left below a frame's content, before the frame's own edge.
const CONTENT_MARGIN_BOTTOM: i32 = 4;

#[derive(Deserialize, Debug)]
struct LayoutDef {
    #[serde(default)]
    toggles: Vec<ToggleDef>,
    #[serde(default)]
    groups: Vec<GroupDef>,
}

#[derive(Deserialize, Debug)]
struct ToggleDef {
    control: String,
    label: String,
    #[serde(default)]
    rules: Vec<String>,
}

#[derive(Deserialize, Debug)]
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
struct ControlDef {
    control: String,
    widget: WidgetKind,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    options: Vec<RadioOption>,
    /// CSS classes applied to the control widget itself.
    #[serde(default)]
    rules: Vec<String>,
    /// CSS classes applied to this control's label widget - independent of
    /// `rules`, since a label and its control can legitimately animate
    /// differently (e.g. a label fades via `opacity` while its control
    /// actually `show`/`hide`s).
    #[serde(rename = "label-rules", default)]
    label_rules: Vec<String>,
}

#[derive(Deserialize, Debug)]
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

/// Parse a HOCON device layout and build the corresponding `gtk::Widget` tree.
pub fn build_ui(conf_str: &str) -> Result<gtk::Widget> {
    let layout: LayoutDef = hocon::de::from_str(conf_str)
        .map_err(|e| anyhow!("Failed to parse layout: {}", e))?;

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_border_width(5);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    if !layout.toggles.is_empty() {
        let bar = build_toggle_bar(&layout.toggles);
        header.pack_start(&bar, false, true, 0);
    }
    header.pack_start(&build_program_name_frame(), true, true, 0);
    root.pack_start(&header, false, true, 0);

    root.pack_start(&build_columns(&layout.groups), true, true, 0);

    root.show_all();
    Ok(root.upcast())
}

fn build_toggle_bar(toggles: &[ToggleDef]) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_widget_name("toggles");
    grid.set_column_homogeneous(true);
    grid.set_valign(gtk::Align::End);

    for (i, t) in toggles.iter().enumerate() {
        let check = gtk::CheckButton::with_label(t.label.as_str());
        check.set_widget_name(&t.control);
        apply_rules(&check, &t.rules);
        grid.attach(&check, i as i32, 1, 1, 1);
    }

    grid
}

fn build_program_name_frame() -> gtk::Frame {
    let entry = gtk::Entry::new();
    entry.set_widget_name("program_name");
    entry.set_valign(gtk::Align::Center);
    apply_content_margins(&entry);

    let frame = gtk::Frame::new(Some("Program"));
    frame.set_label_align(0.0, 0.5);
    frame.set_shadow_type(gtk::ShadowType::EtchedOut);
    frame.add(&entry);

    frame
}

fn build_columns(groups: &[GroupDef]) -> gtk::Box {
    let columns_box = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    columns_box.set_homogeneous(true);

    let max_column = groups.iter().map(|g| g.column).max().unwrap_or(0);
    for column in 0..=max_column {
        let col_box = gtk::Box::new(gtk::Orientation::Vertical, 5);
        for g in groups.iter().filter(|g| g.column == column) {
            col_box.pack_start(&build_group(g), false, true, 0);
        }
        columns_box.pack_start(&col_box, true, true, 0);
    }

    columns_box
}

fn build_group(g: &GroupDef) -> gtk::Frame {
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
        build_control_row(&grid, row as i32, c);
    }

    frame.add(&grid);
    frame
}

fn build_control_row(grid: &gtk::Grid, row: i32, c: &ControlDef) {
    let widget = build_control_widget(c);
    apply_rules(&widget, &c.rules);

    match &c.label {
        Some(label) => {
            let l = gtk::Label::new(Some(label.as_str()));
            l.set_xalign(0.0);
            apply_rules(&l, &c.label_rules);
            grid.attach(&l, 0, row, 1, 1);
            widget.set_hexpand(true);
            grid.attach(&widget, 1, row, 1, 1);
        }
        None => {
            widget.set_hexpand(true);
            grid.attach(&widget, 0, row, 2, 1);
        }
    }
}

fn build_control_widget(c: &ControlDef) -> gtk::Widget {
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
            check.upcast()
        }
        WidgetKind::Combo => {
            let combo = gtk::ComboBoxText::new();
            combo.set_widget_name(&c.control);
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
                b.pack_start(&radio, false, true, 0);
            }
            b.upcast()
        }
        WidgetKind::Entry => {
            let entry = gtk::Entry::new();
            entry.set_widget_name(&c.control);
            entry.set_valign(gtk::Align::Center);
            entry.upcast()
        }
        WidgetKind::Button => {
            let button = gtk::Button::with_label(c.label.as_deref().unwrap_or(""));
            button.set_widget_name(&c.control);
            button.upcast()
        }
    }
}

fn apply_rules<T: IsA<gtk::Widget>>(widget: &T, rules: &[String]) {
    let style = widget.style_context();
    for rule in rules {
        style.add_class(rule);
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

    const CONF: &str = r#"
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
                rules = [ "amp_select=4:show", "amp_select=*:hide" ],
                label-rules = [ "amp_select=4:opacity=1", "amp_select=*:opacity=0" ] }
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
        assert_eq!(drive2.label_rules, vec!["amp_select=4:opacity=1", "amp_select=*:opacity=0"]);

        let reverb = &layout.groups[2];
        assert_eq!(reverb.controls[0].widget, WidgetKind::RadioGroup);
        assert_eq!(reverb.controls[0].options.len(), 2);
    }

    #[test]
    fn builds_widget_tree() {
        gtk::init().expect("gtk::init failed - is a display available?");

        let widget = build_ui(CONF).expect("should build");
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

        // a control's label and widget can carry independent rules
        let drive2 = objs.obj_by_name("drive2").expect("drive2 widget");
        let drive2_classes: Vec<String> = drive2.dynamic_cast_ref::<gtk::Widget>().unwrap()
            .style_context().list_classes().iter().map(|c| c.to_string()).collect();
        assert!(drive2_classes.contains(&"amp_select=4:show".to_string()));
        assert!(!drive2_classes.iter().any(|c| c.contains("opacity")));
    }
}
