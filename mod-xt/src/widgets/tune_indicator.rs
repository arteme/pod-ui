use anyhow::*;
use std::cell::Cell;
use once_cell::sync::Lazy;
use pod_gtk::prelude::gtk::cairo::Context;
use pod_gtk::prelude::subclass::*;

glib::wrapper! {
    pub struct TuneIndicator(ObjectSubclass<TuneIndicatorPriv>)
    @extends gtk::DrawingArea, gtk::Widget;
}

pub struct TuneIndicatorPriv {
    allocation: Cell<gtk::Allocation>,
    indicator: Cell<bool>,
    pos: Cell<f64>
}

const X1_MARGIN: f64 = 5.0;
const X2_MARGIN: f64 = 2.0;
const Y1_MARGIN: f64 = 5.0;
const Y2_MARGIN: f64 = 5.0;

const MID_NOTCH_H: f64 = 10.0;
const NOTCH_H: f64 = 5.0;

const ROMBUS_MARGIN_Y: f64 = 4.0;
const ROMBUS_SCALE_H: f64 = 0.5;

impl TuneIndicatorPriv {
    fn set_pos(&self, value: f64) {
        self.pos.set(value);
        self.instance().queue_draw();
    }

    fn pos(&self) -> f64 {
        self.pos.get()
    }

    fn set_indicator(&self, value: bool) {
        self.indicator.set(value);
        self.instance().queue_draw();
    }

    fn indicator(&self) -> bool {
        self.indicator.get()
    }

    fn allocation_changed(&self, alloc: &gtk::Allocation) {
        self.allocation.set(alloc.clone());
    }

    fn draw(&self, cr: &Context, style: &gtk::StyleContext) -> Result<()> {
        let c = style.color(gtk::StateFlags::NORMAL);

        cr.set_source_rgb(c.red(), c.green(), c.blue());
        cr.set_font_size(32.0);

        let font_extents = cr.font_extents()?;
        println!("{:?}", font_extents);

        let mid_y = self.allocation.get().height() as f64 / 2.0 +
                         font_extents.ascent / 2.0;

        let l_extents = cr.text_extents("♭")?;
        let r_extents = cr.text_extents("♯")?;

        let x1 = l_extents.width + l_extents.x_bearing + X1_MARGIN;
        let x2 = self.allocation.get().width() as f64 - r_extents.width - r_extents.x_bearing - X2_MARGIN;
        let y1 = mid_y - font_extents.ascent + Y1_MARGIN;
        let y2 = mid_y + font_extents.descent - Y2_MARGIN;
        let mid_x = x1 + (x2 - x1) / 2.0;

        cr.move_to(0.0, mid_y);
        cr.show_text("♭")?;

        cr.move_to(x2 + X2_MARGIN, mid_y);
        cr.show_text("♯")?;

        //context.set_source_rgb(0.3,0.3, 0.3);

        cr.set_line_width(1.0);
        cr.move_to(x1, y1);
        cr.line_to(x2, y1);
        cr.line_to(x2, y2);
        cr.line_to(x1, y2);
        cr.line_to(x1, y1);
        cr.stroke()?;

        // indicator position & dimensions
        let pos = self.pos.get();
        let pos = pos.min(1.0).max(-1.0); // clamp to [-1, 1]
        let rh = (y2 - y1) - ROMBUS_MARGIN_Y * 2.0;
        let rw = rh * ROMBUS_SCALE_H;
        let x = mid_x + (x2 - mid_x - rw / 2.0) * pos;
        let y = y1 + (y2 - y1) / 2.0;

        let draw_notch = |x: f64, h: f64| {
            cr.move_to(x, y1 - h);
            cr.line_to(x, y1);
            cr.move_to(x, y2 + h);
            cr.line_to(x, y2);
        };

        // mid notch
        cr.set_line_width(0.5);
        draw_notch(mid_x, MID_NOTCH_H);
        // other notches
        for i in 1 ..= 5 {
            let x = (mid_x - x1 - rw / 2.0) / 5.0 * (i as f64);
            draw_notch(mid_x - x, NOTCH_H);
            draw_notch(mid_x + x, NOTCH_H);
        }
        cr.stroke()?;

        let draw_rombus = |x: f64, y: f64, w: f64, h: f64| {
            cr.move_to(x - w/2.0, y);
            cr.line_to(x, y - h/2.0);
            cr.line_to(x + w/2.0, y);
            cr.line_to(x, y + h/2.0);
            cr.line_to(x - w/2.0, y);
        };

        if self.indicator.get() {
            // indicator
            cr.set_line_width(1.0);
            draw_rombus(x, y, rw, rh);
            cr.stroke()?;
        }


        Ok(())
    }
}

#[glib::object_subclass]
impl ObjectSubclass for TuneIndicatorPriv {
    const NAME: &'static str = "TuneIndicator";
    type Type = TuneIndicator;
    type ParentType = gtk::DrawingArea;

    fn new() -> Self {
        Self {
            allocation: Cell::new(gtk::Allocation::new(0, 0, 0, 0)),
            pos: Cell::new(0.0),
            indicator: Cell::new(false),
        }
    }
}

impl ObjectImpl for TuneIndicatorPriv {
    fn properties() -> &'static [ParamSpec] {
        static PROPERTIES: Lazy<Vec<ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpecDouble::new(
                    "pos",
                    "Position",
                    "Tune indicator position",
                    -1.0,
                    1.0,
                    0.0,
                    glib::ParamFlags::READWRITE
                ),
                glib::ParamSpecBoolean::new(
                    "indicator",
                    "Indicator",
                    "Show indicator",
                    false,
                    glib::ParamFlags::READWRITE
                )
            ]
        });
        PROPERTIES.as_ref()
    }

    fn set_property(&self, _obj: &Self::Type, _id: usize, value: &Value, pspec: &ParamSpec) {
        fn v<'a, T: FromValue<'a>>(value: &'a Value) -> T {
            value.get().expect("type conformity checked by `Object::set_property`")
        }
        match pspec.name() {
            "pos" => self.set_pos(v(value)),
            "indicator" => self.set_indicator(v(value)),
            _ => unimplemented!(),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &ParamSpec) -> Value {
        match pspec.name() {
            "pos" => self.pos().to_value(),
            "indicator" => self.indicator().to_value(),
            _ => unimplemented!(),
        }
    }

    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);
    }
}

impl WidgetImpl for TuneIndicatorPriv {
    fn draw(&self, widget: &Self::Type, cr: &Context) -> Inhibit {
        self.draw(cr, &widget.style_context()).ok();
        Inhibit(true)
    }

    fn preferred_width(&self, _widget: &Self::Type) -> (i32, i32) {
        // TODO: calculate min width for selected font height in a cairo
        //       context, computed x1,x2,y1,y2 and resulting rombus dimensions
        (100, 1000)
    }

    fn size_allocate(&self, widget: &Self::Type, allocation: &gtk::Allocation) {
        self.parent_size_allocate(widget, allocation);
        self.allocation_changed(allocation);
    }
}
impl DrawingAreaImpl for TuneIndicatorPriv {}

impl TuneIndicator {
    pub fn new() -> Self {
        glib::Object::new(&[])
            .expect("Failed to create TuneIndicator")
    }
}

pub trait TuneIndicatorExt {
    fn set_pos(&self, value: Option<f64>);
    fn pos(&self) -> Option<f64>;
}

impl TuneIndicatorExt for TuneIndicator {
    fn set_pos(&self, value: Option<f64>) {
        let p = TuneIndicatorPriv::from_instance(self);
        if let Some(v) = value {
            p.set_pos(v);
            p.set_indicator(true);
        } else {
            p.set_indicator(false);
        }
    }

    fn pos(&self) -> Option<f64> {
        let p = TuneIndicatorPriv::from_instance(self);
        if p.indicator() {
            Some(p.pos())
        } else {
            None
        }
    }
}