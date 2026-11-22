//! One row per player in play order, holding the conditions on that player's hand.

use super::tiles::{self, EditState, Target, TileRow};
use crate::model::{Model, Position};
use crate::theme::{self, Accent};
use eframe::egui::{self, RichText, Ui};
use schafsim_core::game::{Game, Seat};

pub fn seat_label(seat: Seat) -> &'static str {
    match seat {
        Seat::Me => "Ich",
        Seat::Left => "Links",
        Seat::Across => "Gegenüber",
        Seat::Right => "Rechts",
    }
}

pub fn position_label(position: Position, me: Position) -> String {
    let seat = position.seat(me);
    format!("{}. Position · {}", position.number(), seat_label(seat))
}

pub fn show(ui: &mut Ui, model: &mut Model, edit: &mut EditState, game: &Game) {
    let seats_accent = theme::accent(ui, Accent::Seats);
    theme::section_header(
        ui,
        "Blätter",
        "Was über die Hände bekannt ist oder angenommen wird. Leere Hände sind zufällig.",
        seats_accent,
    );
    for position in Position::ALL {
        let seat = position.seat(model.my_position);
        let accent = match seat {
            Seat::Me => theme::accent(ui, Accent::Me),
            _ => seats_accent,
        };
        theme::card(ui, accent, |ui| {
            header(ui, position, seat, accent);
            let blocked = model.cards_elsewhere(seat);
            let row = TileRow {
                target: Target::Seat(seat),
                tiles: &mut model.seat_mut(seat).tiles,
                blocked,
                accent,
            };
            tiles::show(ui, game, edit, row);
        });
        ui.add_space(6.0);
    }
    filter_preview(ui, model);
}

fn header(ui: &mut Ui, position: Position, seat: Seat, accent: egui::Color32) {
    ui.horizontal(|ui| {
        let title = format!("{}. Position", position.number());
        ui.label(RichText::new(title).strong().size(15.0));
        match seat {
            Seat::Me => theme::badge(ui, "Ich", accent),
            _ => ui.label(seat_label(seat)),
        };
        if position == Position::First {
            ui.weak("Vorhand · spielt aus");
        }
    });
}

fn filter_preview(ui: &mut Ui, model: &Model) {
    egui::CollapsingHeader::new("Als Abfragetext")
        .id_salt("seat_filter_text")
        .show(ui, |ui| {
            theme::inset(ui, |ui| {
                let fixed = crate::model::cards_of(&model.seat(Seat::Me).tiles);
                ui.monospace(format!("Feste Karten: [{fixed}]"));
                ui.monospace(format!(
                    "Filter: {}",
                    model.filter().unwrap_or_else(|| "–".into())
                ));
            });
        });
}
