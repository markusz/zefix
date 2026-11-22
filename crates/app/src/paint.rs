//! Card faces and suit symbols, drawn with egui shapes so no image assets are needed.

use eframe::egui::{
    self, Align2, Color32, FontId, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui, Vec2, vec2,
};
use schafsim_core::cards::{Card, Suit};
use std::f32::consts::{FRAC_PI_2, PI};

pub const CARD_SIZE: Vec2 = vec2(34.0, 48.0);
const CARD_FACE: Color32 = Color32::from_rgb(250, 248, 240);
const CARD_INK: Color32 = Color32::from_rgb(40, 40, 40);
const CORNER_RADIUS: f32 = 4.0;

const EICHEL_CAP: Color32 = Color32::from_rgb(110, 76, 47);
const EICHEL_NUT: Color32 = Color32::from_rgb(200, 160, 64);
const GRAS: Color32 = Color32::from_rgb(46, 125, 50);
const GRAS_RIB: Color32 = Color32::from_rgb(27, 80, 30);
const HERZ: Color32 = Color32::from_rgb(198, 40, 40);
const SCHELLEN: Color32 = Color32::from_rgb(230, 170, 20);
const SCHELLEN_DARK: Color32 = Color32::from_rgb(120, 70, 20);
/// Polygon resolution for curved outlines.
const CURVE_STEPS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardLook {
    Normal,
    Selected,
    /// Already used elsewhere; shown dimmed and not clickable.
    Unavailable,
}

/// Draws a suit symbol roughly filling a circle of `radius` around `center`.
pub fn paint_suit(painter: &Painter, center: Pos2, radius: f32, suit: Suit) {
    match suit {
        Suit::Eichel => eichel(painter, center, radius),
        Suit::Gras => gras(painter, center, radius),
        Suit::Herz => herz(painter, center, radius),
        Suit::Schellen => schellen(painter, center, radius),
    }
}

fn eichel(p: &Painter, c: Pos2, r: f32) {
    p.add(Shape::ellipse_filled(
        c + vec2(0.0, 0.25 * r),
        vec2(0.48 * r, 0.68 * r),
        EICHEL_NUT,
    ));
    // Cap: upper half of an ellipse, overlapping the nut.
    let cap_center = c + vec2(0.0, -0.15 * r);
    let cap: Vec<Pos2> = (0..=CURVE_STEPS)
        .map(|i| {
            let a = PI + PI * i as f32 / CURVE_STEPS as f32;
            cap_center + vec2(0.62 * r * a.cos(), 0.45 * r * a.sin())
        })
        .collect();
    p.add(Shape::convex_polygon(cap, EICHEL_CAP, Stroke::NONE));
    p.line_segment(
        [c + vec2(0.0, -0.55 * r), c + vec2(0.12 * r, -0.92 * r)],
        Stroke::new(0.12 * r, EICHEL_CAP),
    );
}

fn gras(p: &Painter, c: Pos2, r: f32) {
    // Leaf outline: width follows a cosine, which keeps the polygon convex.
    let point = |i: usize, side: f32| {
        let y = -1.0 + 2.0 * i as f32 / CURVE_STEPS as f32;
        c + vec2(side * 0.6 * r * (y * FRAC_PI_2).cos(), 0.9 * r * y)
    };
    let mut outline: Vec<Pos2> = (0..=CURVE_STEPS).map(|i| point(i, 1.0)).collect();
    outline.extend((1..CURVE_STEPS).rev().map(|i| point(i, -1.0)));
    p.add(Shape::convex_polygon(outline, GRAS, Stroke::NONE));
    p.line_segment(
        [c + vec2(0.0, -0.6 * r), c + vec2(0.0, 1.1 * r)],
        Stroke::new(0.1 * r, GRAS_RIB),
    );
}

fn herz(p: &Painter, c: Pos2, r: f32) {
    p.circle_filled(c + vec2(-0.45 * r, -0.25 * r), 0.5 * r, HERZ);
    p.circle_filled(c + vec2(0.45 * r, -0.25 * r), 0.5 * r, HERZ);
    p.add(Shape::convex_polygon(
        vec![
            c + vec2(-0.9 * r, -0.08 * r),
            c + vec2(0.9 * r, -0.08 * r),
            c + vec2(0.0, 0.95 * r),
        ],
        HERZ,
        Stroke::NONE,
    ));
}

fn schellen(p: &Painter, c: Pos2, r: f32) {
    let body = 0.8 * r;
    p.circle_filled(c, body, SCHELLEN);
    p.circle_stroke(c, body, Stroke::new(0.08 * r, SCHELLEN_DARK));
    p.line_segment(
        [c + vec2(-0.75 * r, 0.0), c + vec2(0.75 * r, 0.0)],
        Stroke::new(0.14 * r, SCHELLEN_DARK),
    );
    p.circle_filled(c + vec2(0.0, 0.42 * r), 0.13 * r, SCHELLEN_DARK);
    p.circle_filled(c + vec2(0.0, -0.92 * r), 0.12 * r, SCHELLEN_DARK);
}

/// A clickable card face; `scale` 1.0 is `CARD_SIZE`.
pub fn card(ui: &mut Ui, card: Card, look: CardLook, scale: f32) -> Response {
    let sense = match look {
        CardLook::Unavailable => Sense::hover(),
        CardLook::Normal | CardLook::Selected => Sense::click(),
    };
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE * scale, sense);
    if ui.is_rect_visible(rect) {
        paint_card(ui, rect, card, look, response.hovered());
    }
    response
}

fn paint_card(ui: &Ui, rect: Rect, card: Card, look: CardLook, hovered: bool) {
    let painter = ui.painter();
    let visuals = ui.visuals();
    let stroke = match look {
        CardLook::Selected => Stroke::new(2.5, visuals.selection.bg_fill),
        _ if hovered => Stroke::new(1.5, visuals.widgets.hovered.fg_stroke.color),
        // Fixed cards get a solid black edge; rule cards a dotted one.
        _ => Stroke::new(1.5, CARD_INK),
    };
    painter.rect_filled(rect, CORNER_RADIUS, CARD_FACE);
    painter.rect_stroke(rect, CORNER_RADIUS, stroke, StrokeKind::Inside);
    let symbol_center = rect.center_top() + vec2(0.0, rect.height() * 0.33);
    paint_suit(painter, symbol_center, rect.width() * 0.34, card.suit());
    painter.text(
        rect.center_bottom() - vec2(0.0, rect.height() * 0.06),
        Align2::CENTER_BOTTOM,
        card.rank().symbol(),
        FontId::proportional(rect.height() * 0.32),
        CARD_INK,
    );
    if look == CardLook::Unavailable {
        painter.rect_filled(rect, CORNER_RADIUS, visuals.panel_fill.gamma_multiply(0.75));
    }
}

/// Inline suit symbol at text height, for labels.
pub fn suit_icon(ui: &mut Ui, suit: Suit) -> Response {
    let size = ui.text_style_height(&egui::TextStyle::Body);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint_suit(ui.painter(), rect.center(), size * 0.42, suit);
    response
}

/// Hand rows scale their cards to the available width, within these bounds.
pub const MIN_HAND_SCALE: f32 = 1.2;
pub const MAX_HAND_SCALE: f32 = 2.6;
const DOT_SPACING: f32 = 4.0;
const RULE_FACE_ALPHA: f32 = 0.8;
const DOT_RADIUS: f32 = 1.0;
const DASH_LENGTH: f32 = 6.0;
const DASH_GAP: f32 = 4.0;
/// Points per rounded corner of an outline path.
const CORNER_STEPS: usize = 4;

/// What a rule card shows above its badge.
pub enum RuleSymbol<'a> {
    Suit(Suit),
    /// A rank across all suits, e.g. "O" for Ober.
    Rank(&'static str),
    /// Trump: the trump suit, or the trump rank where no suit is trump (Wenz, Geier).
    Trumpf {
        suit: Option<Suit>,
        rank: &'static str,
    },
    /// Several suits, e.g. all non-trump suits for "Farbe".
    Suits(&'a [Suit]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Negation {
    Plain,
    /// "keine": the symbol is struck through.
    Struck,
}

pub struct RuleFace<'a> {
    pub symbol: RuleSymbol<'a>,
    pub badge: String,
    pub negation: Negation,
}

fn rounded_outline(rect: Rect, radius: f32) -> Vec<Pos2> {
    let corners = [
        (rect.right_top() + vec2(-radius, radius), -FRAC_PI_2),
        (rect.right_bottom() + vec2(-radius, -radius), 0.0),
        (rect.left_bottom() + vec2(radius, -radius), FRAC_PI_2),
        (rect.left_top() + vec2(radius, radius), PI),
    ];
    let mut points: Vec<Pos2> = corners
        .iter()
        .flat_map(|&(center, start)| {
            (0..=CORNER_STEPS).map(move |i| {
                let a = start + FRAC_PI_2 * i as f32 / CORNER_STEPS as f32;
                center + radius * vec2(a.cos(), a.sin())
            })
        })
        .collect();
    if let Some(&first) = points.first() {
        points.push(first);
    }
    points
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiniCard {
    Solid,
    Dotted,
}

/// Tiny card outline for button labels, in the same edge style as the cards they add.
pub fn mini_card(painter: &Painter, rect: Rect, style: MiniCard, color: Color32) {
    const RADIUS: f32 = 1.5;
    match style {
        MiniCard::Solid => {
            painter.rect_stroke(rect, RADIUS, Stroke::new(1.5, color), StrokeKind::Inside);
        }
        MiniCard::Dotted => {
            let path = rounded_outline(rect, RADIUS);
            painter.extend(Shape::dotted_line(&path, color, 2.5, 0.75));
        }
    }
}

/// Dashed card outline for the "add" placeholder.
pub fn dashed_outline(painter: &Painter, rect: Rect, color: Color32) {
    let path = rounded_outline(rect, CORNER_RADIUS);
    painter.extend(Shape::dashed_line(
        &path,
        Stroke::new(1.5, color),
        DASH_LENGTH,
        DASH_GAP,
    ));
}

/// A rule ("at least 3 Gras") drawn as a card with a dotted edge and a count badge.
pub fn rule_card(
    ui: &mut Ui,
    face: &RuleFace<'_>,
    look: CardLook,
    scale: f32,
    accent: Color32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(CARD_SIZE * scale, Sense::click());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        // Slightly translucent face: a rule stands for cards, it is not one.
        painter.rect_filled(
            rect,
            CORNER_RADIUS,
            CARD_FACE.gamma_multiply(RULE_FACE_ALPHA),
        );
        let edge = if response.hovered() {
            ui.visuals().widgets.hovered.fg_stroke.color
        } else {
            ui.visuals().strong_text_color()
        };
        let path = rounded_outline(rect, CORNER_RADIUS);
        painter.extend(Shape::dotted_line(&path, edge, DOT_SPACING, DOT_RADIUS));
        if look == CardLook::Selected {
            let ring = rect.expand(2.5);
            painter.rect_stroke(
                ring,
                CORNER_RADIUS + 2.0,
                Stroke::new(2.0, accent),
                StrokeKind::Outside,
            );
        }
        paint_rule_symbol(painter, rect, face);
        paint_badge(painter, rect, &face.badge, accent);
    }
    response
}

/// The symbol fills the area above the badge (top ~70 % of the card).
fn paint_rule_symbol(painter: &Painter, rect: Rect, face: &RuleFace<'_>) {
    let (w, h) = (rect.width(), rect.height());
    let at = |y: f32| rect.center_top() + vec2(0.0, h * y);
    let letter = |text: &str, pos: Pos2, size: f32| {
        painter.text(
            pos,
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(size),
            CARD_INK,
        );
    };
    match &face.symbol {
        RuleSymbol::Suit(suit) => paint_suit(painter, at(0.36), w * 0.4, *suit),
        RuleSymbol::Rank(rank) => {
            letter(rank, at(0.27), h * 0.4);
            suit_row(painter, at(0.57), w, &Suit::ALL);
        }
        RuleSymbol::Trumpf { suit, rank } => {
            match suit {
                Some(suit) => paint_suit(painter, at(0.3), w * 0.32, *suit),
                None => letter(rank, at(0.3), h * 0.38),
            }
            letter("Trumpf", at(0.6), h * 0.15);
        }
        RuleSymbol::Suits(suits) => suit_grid(painter, at(0.38), rect.size(), suits),
    }
    if face.negation == Negation::Struck {
        let center = at(0.36);
        let d = vec2(w * 0.4, h * 0.26);
        painter.line_segment([center - d, center + d], Stroke::new(2.5, HERZ));
    }
}

fn suit_row(painter: &Painter, center: Pos2, card_width: f32, suits: &[Suit]) {
    let step = card_width * 0.24;
    let radius = card_width * 0.1;
    let first = center.x - step * (suits.len().saturating_sub(1)) as f32 / 2.0;
    for (i, &suit) in suits.iter().enumerate() {
        paint_suit(
            painter,
            Pos2::new(first + step * i as f32, center.y),
            radius,
            suit,
        );
    }
}

/// Up to four suits in two rows of two; an odd last one is centred.
fn suit_grid(painter: &Painter, center: Pos2, card: Vec2, suits: &[Suit]) {
    const COLUMNS: usize = 2;
    let (dx, dy) = (card.x * 0.22, card.y * 0.14);
    let radius = card.x * 0.17;
    let rows = suits.len().div_ceil(COLUMNS);
    for (row, chunk) in suits.chunks(COLUMNS).enumerate() {
        let y = center.y + dy * (2.0 * row as f32 - (rows - 1) as f32);
        for (col, &suit) in chunk.iter().enumerate() {
            let x = center.x + dx * (2.0 * col as f32 - (chunk.len() - 1) as f32);
            paint_suit(painter, Pos2::new(x, y), radius, suit);
        }
    }
}

fn paint_badge(painter: &Painter, rect: Rect, text: &str, fill: Color32) {
    let font = FontId::proportional(rect.height() * 0.2);
    let ink = if fill.intensity() > 0.55 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    let galley = painter.layout_no_wrap(text.to_string(), font, ink);
    let size = galley.size() + vec2(8.0, 2.0);
    let center = rect.center_bottom() - vec2(0.0, rect.height() * 0.16);
    let badge = Rect::from_center_size(center, size);
    painter.rect_filled(badge, size.y / 2.0, fill);
    painter.galley(badge.center() - galley.size() / 2.0, galley, ink);
}
