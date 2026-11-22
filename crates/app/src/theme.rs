//! Look and feel: surfaces in three elevations, and one accent per area so seats, queries and
//! results read as separate parts. Accents are slots of a validated categorical palette, stepped
//! separately for dark and light surfaces; identity never rests on colour alone (every area
//! also has a heading).

use eframe::egui::{
    self, Button, Color32, CornerRadius, Frame, Margin, Response, RichText, Stroke, Theme, Ui,
    Vec2, vec2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accent {
    Seats,
    Me,
    Queries,
    Results,
}

struct Tokens {
    panel: Color32,
    card: Color32,
    inset: Color32,
    border: Color32,
    border_strong: Color32,
    button: Color32,
    button_hover: Color32,
    text: Color32,
    text_secondary: Color32,
    selection: Color32,
}

const DARK: Tokens = Tokens {
    panel: Color32::from_rgb(0x14, 0x14, 0x13),
    card: Color32::from_rgb(0x22, 0x22, 0x20),
    inset: Color32::from_rgb(0x10, 0x10, 0x0f),
    border: Color32::from_rgb(0x3a, 0x39, 0x35),
    border_strong: Color32::from_rgb(0x6a, 0x69, 0x63),
    button: Color32::from_rgb(0x30, 0x30, 0x2d),
    button_hover: Color32::from_rgb(0x3d, 0x3d, 0x39),
    text: Color32::from_rgb(0xf0, 0xef, 0xec),
    text_secondary: Color32::from_rgb(0xc3, 0xc2, 0xb7),
    selection: Color32::from_rgb(0x4a, 0x42, 0x8c),
};

const LIGHT: Tokens = Tokens {
    panel: Color32::from_rgb(0xec, 0xeb, 0xe6),
    card: Color32::from_rgb(0xfc, 0xfc, 0xfb),
    inset: Color32::from_rgb(0xf3, 0xf2, 0xee),
    border: Color32::from_rgb(0xd6, 0xd4, 0xcc),
    border_strong: Color32::from_rgb(0x9a, 0x98, 0x90),
    button: Color32::from_rgb(0xe6, 0xe5, 0xdf),
    button_hover: Color32::from_rgb(0xdb, 0xd9, 0xd2),
    text: Color32::from_rgb(0x0b, 0x0b, 0x0b),
    text_secondary: Color32::from_rgb(0x52, 0x51, 0x4e),
    selection: Color32::from_rgb(0xd8, 0xd3, 0xf6),
};

const CORNER: u8 = 6;
const CARD_CORNER: u8 = 8;
const CHIP_CORNER: u8 = 12;
const STRIPE_WIDTH: f32 = 4.0;
const CARD_MARGIN: i8 = 10;
const INSET_MARGIN: i8 = 8;

pub fn accent(ui: &Ui, accent: Accent) -> Color32 {
    let dark = ui.visuals().dark_mode;
    let (light_hex, dark_hex) = match accent {
        // Light steps darker than the palette's: aqua and yellow fall below 3:1 as button text.
        Accent::Seats => (0x0f_8a_60, 0x19_9e_70),
        Accent::Me => (0xa8_72_00, 0xc9_85_00),
        Accent::Queries => (0x62_50_d6, 0x90_85_e9),
        Accent::Results => (0x2a_78_d6, 0x39_87_e5),
    };
    hex(if dark { dark_hex } else { light_hex })
}

fn hex(rgb: u32) -> Color32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color32::from_rgb(r, g, b)
}

pub fn install(ctx: &egui::Context) {
    ctx.style_mut_of(Theme::Dark, |style| configure(style, &DARK));
    ctx.style_mut_of(Theme::Light, |style| configure(style, &LIGHT));
}

fn configure(style: &mut egui::Style, t: &Tokens) {
    style.spacing.item_spacing = vec2(8.0, 6.0);
    style.spacing.button_padding = vec2(8.0, 4.0);
    let v = &mut style.visuals;
    v.panel_fill = t.panel;
    v.window_fill = t.card;
    v.faint_bg_color = t.card;
    v.extreme_bg_color = t.inset;
    v.code_bg_color = t.inset;
    v.text_edit_bg_color = Some(t.inset);
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, t.text);
    for (state, fill, border) in [
        (&mut w.inactive, t.button, t.border),
        (&mut w.hovered, t.button_hover, t.border_strong),
        (&mut w.active, t.button_hover, t.text_secondary),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = Stroke::new(1.0, border);
        state.corner_radius = CornerRadius::same(CORNER);
    }
    w.inactive.fg_stroke = Stroke::new(1.0, t.text);
    w.hovered.fg_stroke = Stroke::new(1.5, t.text);
    w.active.fg_stroke = Stroke::new(1.5, t.text);
    v.weak_text_color = Some(t.text_secondary);
    // Selected toggles (position, mode, scope) wear a muted violet, not egui's default blue.
    v.selection.bg_fill = t.selection;
    v.selection.stroke = Stroke::new(1.0, t.text);
}

/// Elevated surface with an accent stripe on the left edge.
pub fn card<R>(ui: &mut Ui, accent: Color32, add: impl FnOnce(&mut Ui) -> R) -> R {
    let visuals = ui.visuals();
    let frame = Frame::new()
        .fill(visuals.window_fill)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(CARD_CORNER))
        .inner_margin(Margin {
            left: CARD_MARGIN + STRIPE_WIDTH as i8,
            right: CARD_MARGIN,
            top: CARD_MARGIN,
            bottom: CARD_MARGIN,
        });
    let response = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
    let rect = response.response.rect;
    let stripe = egui::Rect::from_min_size(rect.min, vec2(STRIPE_WIDTH, rect.height()));
    let radius = CornerRadius {
        nw: CARD_CORNER,
        sw: CARD_CORNER,
        ne: 0,
        se: 0,
    };
    ui.painter().rect_filled(stripe, radius, accent);
    response.inner
}

/// Raised bar for the global settings.
pub fn top_bar_frame(ui: &Ui) -> Frame {
    let visuals = ui.visuals();
    Frame::new()
        .fill(visuals.window_fill)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .inner_margin(Margin::symmetric(CARD_MARGIN, INSET_MARGIN))
}

/// Recessed surface for editors, pickers and code.
pub fn inset<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(CornerRadius::same(CORNER))
        .inner_margin(Margin::same(INSET_MARGIN))
        .show(ui, add)
        .inner
}

pub fn section_header(ui: &mut Ui, title: &str, subtitle: &str, accent: Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(3), accent);
        ui.heading(RichText::new(title).strong());
    });
    if !subtitle.is_empty() {
        ui.weak(subtitle);
    }
    ui.add_space(4.0);
}

/// An outlined action button in the area's accent, visibly distinct from data tiles.
pub fn action(ui: &mut Ui, text: &str, accent: Color32) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(accent).strong())
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, accent)),
    )
}

pub fn danger(ui: &mut Ui, text: &str) -> Response {
    let color = ui.visuals().error_fg_color;
    ui.add(
        Button::new(RichText::new(text).color(color))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, color.gamma_multiply(0.6))),
    )
}

/// A filled label, e.g. "Ich" or "Vorhand".
pub fn badge(ui: &mut Ui, text: &str, fill: Color32) -> Response {
    let ink = if fill.intensity() > 0.55 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    ui.add(
        Button::new(RichText::new(text).color(ink).strong())
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(CHIP_CORNER))
            .sense(egui::Sense::hover()),
    )
}
