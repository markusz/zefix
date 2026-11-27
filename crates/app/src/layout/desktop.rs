//! Wide screens: one settings bar, seats on the left, queries and result cards on the right.

use super::controls::{self, brand, position_picker, run_settings};
use crate::app::ZefixApp;
use crate::model::GameSetup;
use crate::paint;
use eframe::egui::{self, ScrollArea, Ui};
use zefix_core::game::Game;

const SEAT_PANEL_WIDTH: f32 = 760.0;

pub fn top_bar(app: &mut ZefixApp, ui: &mut Ui) {
    ui.horizontal_wrapped(|ui| {
        brand(ui);
        ui.separator();
        controls::game_choice(ui, &mut app.model.game);
        suit_buttons(ui, &mut app.model.game);
        ui.separator();
        position_picker(ui, &mut app.model);
        ui.separator();
        run_settings(ui, &mut app.settings);
        ui.separator();
        if ui
            .button("Teilen")
            .on_hover_text("Link oder TOML kopieren, Szenario importieren")
            .clicked()
        {
            app.share.open = true;
        }
    });
}

pub fn body(app: &mut ZefixApp, ui: &mut Ui, game: &Game) {
    egui::Panel::left("seats")
        .resizable(true)
        .default_size(SEAT_PANEL_WIDTH)
        .show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| app.seats(ui, game));
        });
    egui::CentralPanel::default().show(ui, |ui| {
        ScrollArea::vertical().show(ui, |ui| {
            app.queries(ui, game, None);
            ui.separator();
            app.results(ui);
        });
    });
}

/// Symbol and name per suit, after a label saying what the suit means in this game.
fn suit_buttons(ui: &mut Ui, setup: &mut GameSetup) {
    let suits = setup.choice.suits();
    if suits.is_empty() {
        return;
    }
    ui.label(setup.choice.suit_label());
    for &suit in suits {
        let selected = setup.suit == suit;
        let response = ui
            .scope(|ui| {
                paint::suit_icon(ui, suit);
                ui.selectable_label(selected, suit.display_name())
            })
            .inner;
        if response.clicked() {
            setup.suit = suit;
        }
    }
}
