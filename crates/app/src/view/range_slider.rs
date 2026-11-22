//! Integer slider with two handles for "between min and max". egui only ships single-value
//! sliders. Every integer gets a tick with its number, since the ranges are short (≤ 8).

use eframe::egui::{
    Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Response, Sense, Stroke, Ui, pos2, vec2,
};

const WIDTH: f32 = 220.0;
const HEIGHT: f32 = 34.0;
const HANDLE_RADIUS: f32 = 8.0;
const RAIL_HEIGHT: f32 = 4.0;
const TICK_FONT_SIZE: f32 = 11.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handle {
    Min,
    Max,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Bounds {
    min: u8,
    max: u8,
}

struct Scale {
    left: f32,
    width: f32,
    cap: u8,
}

impl Scale {
    fn x(&self, v: u8) -> f32 {
        self.left + self.width * f32::from(v) / f32::from(self.cap.max(1))
    }

    fn value_at(&self, x: f32) -> u8 {
        let t = ((x - self.left) / self.width).clamp(0.0, 1.0);
        (t * f32::from(self.cap)).round() as u8
    }
}

pub fn range_slider(ui: &mut Ui, min: &mut u8, max: &mut u8, cap: u8, accent: Color32) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(vec2(WIDTH, HEIGHT), Sense::click_and_drag());
    let scale = Scale {
        left: rect.left() + HANDLE_RADIUS,
        width: rect.width() - 2.0 * HANDLE_RADIUS,
        cap,
    };
    if let Some(pos) = response.interact_pointer_pos() {
        let before = Bounds {
            min: *min,
            max: *max,
        };
        let handle = active_handle(ui, response.id, pos, &scale, before);
        let v = scale.value_at(pos.x);
        match handle {
            Handle::Min => *min = v.min(*max),
            Handle::Max => *max = v.max(*min),
        }
        if *min != before.min || *max != before.max {
            response.mark_changed();
        }
    }
    if ui.is_rect_visible(rect) {
        let bounds = Bounds {
            min: *min,
            max: *max,
        };
        paint(ui, rect, &scale, bounds, accent);
    }
    response
}

/// The handle grabbed on press stays active for the whole drag, even when the two cross paths.
fn active_handle(ui: &Ui, id: Id, pos: Pos2, scale: &Scale, b: Bounds) -> Handle {
    let pressed = ui.input(|i| i.pointer.any_pressed());
    let stored = ui.data(|d| d.get_temp::<Handle>(id));
    match stored {
        Some(handle) if !pressed => handle,
        _ => {
            let dmin = (pos.x - scale.x(b.min)).abs();
            let dmax = (pos.x - scale.x(b.max)).abs();
            // Equal distance (handles on top of each other): the side of the pointer decides.
            let handle = if dmin < dmax || (dmin == dmax && pos.x < scale.x(b.min)) {
                Handle::Min
            } else {
                Handle::Max
            };
            ui.data_mut(|d| d.insert_temp(id, handle));
            handle
        }
    }
}

fn paint(ui: &Ui, rect: Rect, scale: &Scale, b: Bounds, accent: Color32) {
    let Bounds { min, max } = b;
    let painter = ui.painter();
    let visuals = ui.visuals();
    let y = rect.top() + HANDLE_RADIUS + 1.0;
    let rail = |from: f32, to: f32| {
        Rect::from_min_max(
            pos2(from, y - RAIL_HEIGHT / 2.0),
            pos2(to, y + RAIL_HEIGHT / 2.0),
        )
    };
    let radius = CornerRadius::same(2);
    painter.rect_filled(
        rail(scale.x(0), scale.x(scale.cap)),
        radius,
        visuals.widgets.inactive.bg_fill,
    );
    painter.rect_filled(rail(scale.x(min), scale.x(max)), radius, accent);
    for v in 0..=scale.cap {
        let selected = (min..=max).contains(&v);
        let color = if selected {
            visuals.strong_text_color()
        } else {
            visuals.weak_text_color()
        };
        painter.text(
            pos2(scale.x(v), rect.bottom()),
            Align2::CENTER_BOTTOM,
            v.to_string(),
            FontId::proportional(TICK_FONT_SIZE),
            color,
        );
    }
    let ring = Stroke::new(2.0, visuals.window_fill);
    for v in [min, max] {
        let center = pos2(scale.x(v), y);
        painter.circle(center, HANDLE_RADIUS, accent, ring);
    }
}
