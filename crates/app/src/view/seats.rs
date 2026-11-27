//! One row per player in play order, holding the conditions on that player's hand.

use super::tiles::{self, EditState, Target, TileRow};
use crate::model::{Model, Position};
use crate::theme::{self, Accent};
use eframe::egui::{self, RichText, TextEdit, Ui};
use zefix_core::game::{Game, Seat};

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

/// `filter_error` is a compile error in the combined filter, rendered with a caret.
pub fn show(
    ui: &mut Ui,
    model: &mut Model,
    edit: &mut EditState,
    game: &Game,
    filter_error: Option<&str>,
) {
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
    // Only an imported or shared condition is worth showing where code is hidden.
    let has_condition = !model.filter_text.trim().is_empty() || filter_error.is_some();
    if theme::density(ui).shows_code() || has_condition {
        extra_condition(ui, model, filter_error);
    }
    if theme::density(ui).shows_code() {
        filter_preview(ui, model, game);
    }
}

/// Free-text condition on the deal, for what tiles cannot say (e.g. an `or`).
fn extra_condition(ui: &mut Ui, model: &mut Model, error: Option<&str>) {
    egui::CollapsingHeader::new("Weitere Bedingung")
        .id_salt("extra_condition")
        // Kept open while it holds something, so an imported condition is never hidden.
        .open((!model.filter_text.trim().is_empty() || error.is_some()).then_some(true))
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(&mut model.filter_text)
                    .code_editor()
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .hint_text("z. B. left.frei(gras) or right.has(SO)"),
            );
            if let Some(error) = error {
                theme::inset(ui, |ui| {
                    ui.label(
                        RichText::new(error)
                            .monospace()
                            .color(ui.visuals().error_fg_color),
                    );
                });
            }
        });
}

fn filter_preview(ui: &mut Ui, model: &Model, game: &Game) {
    egui::CollapsingHeader::new("Als Abfragetext")
        .id_salt("seat_filter_text")
        .show(ui, |ui| {
            theme::inset(ui, |ui| {
                let fixed = crate::model::cards_of(&model.seat(Seat::Me).tiles);
                ui.monospace(format!("Feste Karten: [{fixed}]"));
                ui.monospace(format!(
                    "Filter: {}",
                    model.filter(game).unwrap_or_else(|| "–".into())
                ));
            });
        });
}
