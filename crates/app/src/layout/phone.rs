//! Narrow screens: a short settings bar, then seats or queries as tabs. Results sit inside each
//! query, the run length and progress below them. No share button: the browser's own share
//! sheet sends the address, which holds the state.

use super::controls::{self, brand, position_picker, run_settings};
use crate::app::ZefixApp;
use crate::model::GameSetup;
use crate::paint;
use crate::view::results;
use eframe::egui::{self, ScrollArea, Ui, Vec2};
use zefix_core::cards::Suit;
use zefix_core::game::Game;

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

/// One group per row: separators in a wrapped row would stretch to the full bar height.
pub fn top_bar(app: &mut ZefixApp, ui: &mut Ui) {
    ui.horizontal(brand);
    ui.horizontal_wrapped(|ui| {
        controls::game_choice(ui, &mut app.model.game);
        suit_icons(ui, &mut app.model.game);
    });
    ui.horizontal_wrapped(|ui| position_picker(ui, &mut app.model));
}

pub fn body(app: &mut ZefixApp, ui: &mut Ui, game: &Game) {
    egui::Panel::top("tabs").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for tab in Tab::ALL {
                ui.selectable_value(&mut app.tab, tab, tab.label());
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
