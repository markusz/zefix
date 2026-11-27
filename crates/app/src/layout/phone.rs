//! Narrow screens: a short settings bar, then seats or queries as tabs. Results sit inside each
//! query, the run length and progress below them. No share button: the browser's own share
//! sheet sends the address, which holds the state.

use super::controls::{self, brand, position_picker, run_settings};
use crate::app::ZefixApp;
use crate::model::GameSetup;
use crate::paint;
use crate::theme::{self, Accent};
use crate::view::results;
use eframe::egui::{self, ScrollArea, Ui, Vec2};
use zefix_core::cards::Suit;
use zefix_core::game::Game;

const TAB_HEIGHT: f32 = 34.0;
const TAB_CORNER: u8 = 8;
const TAB_ICON: f32 = 18.0;
const TAB_ICON_GAP: f32 = 6.0;
const SUIT_BUTTON_EXTRA: f32 = 6.0;
const SUIT_BUTTON_CORNER: u8 = 6;
/// Symbol radius as a share of the button height.
const SUIT_ICON_SHARE: f32 = 0.36;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Tab {
    #[default]
    Seats,
    Queries,
}

impl Tab {
    const ALL: [Tab; 2] = [Tab::Seats, Tab::Queries];

    fn label(self) -> &'static str {
        match self {
            Tab::Seats => "Blätter",
            Tab::Queries => "Abfragen",
        }
    }
}

/// The brand on a tinted strip of its own, then one settings group per row: separators in a
/// wrapped row would stretch to the full bar height.
pub fn top_bar(app: &mut ZefixApp, ui: &mut Ui) {
    egui::Panel::top("brand")
        .frame(theme::brand_bar_frame(ui))
        .show(ui, |ui| ui.horizontal(brand));
    egui::Panel::top("settings")
        .frame(theme::top_bar_frame(ui))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                controls::game_choice(ui, &mut app.model.game);
                suit_icons(ui, &mut app.model.game);
            });
            ui.horizontal_wrapped(|ui| position_picker(ui, &mut app.model));
        });
}

pub fn body(app: &mut ZefixApp, ui: &mut Ui, game: &Game) {
    egui::Panel::top("tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            let count = Tab::ALL.len() as f32;
            let width = (ui.available_width() - spacing * (count - 1.0)) / count;
            for tab in Tab::ALL {
                if tab_button(ui, tab, app.tab == tab, width).clicked() {
                    app.tab = tab;
                }
            }
        });
    });
    egui::CentralPanel::default().show(ui, |ui| {
        ScrollArea::vertical()
            .id_salt(app.tab)
            .show(ui, |ui| match app.tab {
                Tab::Seats => app.seats(ui, game),
                Tab::Queries => {
                    let inline = results::Inline::of(&app.runner);
                    app.queries(ui, game, inline.as_ref());
                    ui.separator();
                    ui.horizontal_wrapped(|ui| run_settings(ui, &mut app.settings));
                    results::status_line(ui, &app.runner);
                }
            });
    });
}

/// Half the width each, icon and name centred, so the tabs read as two big buttons.
fn tab_button(ui: &mut Ui, tab: Tab, selected: bool, width: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, TAB_HEIGHT), egui::Sense::click());
    let visuals = ui.visuals();
    let state = if response.hovered() {
        &visuals.widgets.hovered
    } else {
        &visuals.widgets.inactive
    };
    let fill = if selected {
        visuals.selection.bg_fill
    } else {
        state.weak_bg_fill
    };
    let corner = egui::CornerRadius::same(TAB_CORNER);
    let painter = ui.painter();
    painter.rect(
        rect,
        corner,
        fill,
        state.bg_stroke,
        egui::StrokeKind::Inside,
    );
    tab_content(ui, rect, tab, selected, fill);
    response
}

/// Icon in the area's accent, then the name, centred together.
fn tab_content(ui: &Ui, rect: egui::Rect, tab: Tab, selected: bool, fill: egui::Color32) {
    let mut text = egui::RichText::new(tab.label());
    if selected {
        text = text.strong();
    }
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let left = rect.center().x - (TAB_ICON + TAB_ICON_GAP + galley.size().x) / 2.0;
    let icon = egui::Rect::from_center_size(
        egui::pos2(left + TAB_ICON / 2.0, rect.center().y),
        Vec2::splat(TAB_ICON),
    );
    let painter = ui.painter();
    match tab {
        Tab::Seats => paint::cards_icon(painter, icon, theme::accent(ui, Accent::Seats), fill),
        Tab::Queries => paint::lens_icon(painter, icon, theme::accent(ui, Accent::Queries)),
    }
    let text_pos = egui::pos2(
        left + TAB_ICON + TAB_ICON_GAP,
        rect.center().y - galley.size().y / 2.0,
    );
    painter.galley(text_pos, galley, ui.visuals().text_color());
}

/// Bare suit symbols, so they fit beside the game dropdown; the name is on hover.
fn suit_icons(ui: &mut Ui, setup: &mut GameSetup) {
    for &suit in setup.choice.suits() {
        let hint = format!("{}: {}", setup.choice.suit_label(), suit.display_name());
        if suit_button(ui, suit, setup.suit == suit)
            .on_hover_text(hint)
            .clicked()
        {
            setup.suit = suit;
        }
    }
}

fn suit_button(ui: &mut Ui, suit: Suit, selected: bool) -> egui::Response {
    // As tall as the game dropdown beside it, so both share one centre line.
    let height = ui.spacing().interact_size.y;
    let size = Vec2::new(height + SUIT_BUTTON_EXTRA, height);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let visuals = ui.visuals();
    let fill = if selected {
        visuals.selection.bg_fill
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(SUIT_BUTTON_CORNER), fill);
    paint::paint_suit(ui.painter(), rect.center(), size.y * SUIT_ICON_SHARE, suit);
    response
}
