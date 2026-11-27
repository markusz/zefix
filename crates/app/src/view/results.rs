//! Live results: one probability bar with 95 % interval per query, plus example deals.

use super::seats::position_label;
use crate::format;
use crate::model::Position;
use crate::paint::{self, CardLook};
use crate::runner::{Runner, Shown, Status};
use crate::theme::{self, Accent};
use eframe::egui::{self, ProgressBar, RichText, Ui};
use zefix_core::lang::eval::Deal;
use zefix_core::scenario::{ExprSite, ScenarioError};
use zefix_core::stats::{self, QueryResult, ratio};

const INLINE_PERCENT_SIZE: f32 = 16.0;
/// Enough at a glance; the browser's short runs leave the second decimal noise anyway.
const INLINE_PERCENT_DIGITS: usize = 1;
const EXAMPLE_CARD_SCALE: f32 = 0.6;
const EXAMPLE_LABEL_WIDTH: f32 = 150.0;
/// Opacity of results that belong to an older model.
const STALE_OPACITY: f32 = 0.45;

/// Results to show inside each query's own box (phones), matched by query name so a result
/// stays with its query while queries are added or removed.
pub struct Inline {
    results: Vec<QueryResult>,
    stale: bool,
}

impl Inline {
    pub fn of(runner: &Runner) -> Option<Inline> {
        let shown = runner.shown.as_ref()?;
        Some(Inline {
            results: stats::results(&shown.scenario, &shown.snapshot.outcome),
            stale: runner.is_stale(),
        })
    }

    /// Just the probability, as a chip in the bottom right corner.
    pub fn show_for(&self, ui: &mut Ui, name: &str) {
        let Some(result) = self.results.iter().find(|r| r.name == name) else {
            return;
        };
        ui.scope(|ui| {
            if self.stale {
                ui.set_opacity(STALE_OPACITY);
            }
            // One row high: a bare right-to-left layout would take the rest of the card.
            let row = egui::vec2(ui.available_width(), 0.0);
            let layout = egui::Layout::right_to_left(egui::Align::Center);
            ui.allocate_ui_with_layout(row, layout, |ui| {
                let percent = format::percent_to(result.estimate.p, INLINE_PERCENT_DIGITS);
                let text = RichText::new(percent).size(INLINE_PERCENT_SIZE);
                theme::chip(ui, text, theme::accent(ui, Accent::Results));
            });
        });
    }
}

/// Progress or what went wrong; nothing once the numbers are in.
pub fn status_line(ui: &mut Ui, runner: &Runner) {
    match &runner.status {
        Status::Running => progress(ui, runner),
        Status::Finished | Status::Stopped => {}
        status => problem(ui, status),
    }
}

pub fn show(ui: &mut Ui, runner: &mut Runner, me: Position) {
    let accent = theme::accent(ui, Accent::Results);
    theme::section_header(ui, "Ergebnis", "", accent);
    ui.horizontal(|ui| {
        match runner.status {
            Status::Running => {
                if theme::action(ui, "Stopp", accent).clicked() {
                    runner.stop();
                }
            }
            // A finished run would only repeat itself: the seed is fixed.
            Status::Stopped | Status::Failed(_) => {
                if theme::action(ui, "Neu starten", accent).clicked() {
                    runner.restart();
                }
            }
            Status::Finished | Status::Invalid(_) => {}
        }
        status(ui, runner);
    });
    ui.add_space(4.0);
    let stale = runner.is_stale();
    let Some(shown) = &runner.shown else { return };
    if stale {
        ui.set_opacity(STALE_OPACITY);
    }
    let results = stats::results(&shown.scenario, &shown.snapshot.outcome);
    for (i, result) in results.iter().enumerate() {
        let examples = shown
            .snapshot
            .outcome
            .tallies
            .get(i)
            .map_or(&[][..], |t| &t.examples[..]);
        result_card(ui, i, result, examples, shown, me);
        ui.add_space(6.0);
    }
}

fn status(ui: &mut Ui, runner: &Runner) {
    match &runner.status {
        Status::Running => progress(ui, runner),
        Status::Finished | Status::Stopped => {
            if let Some(shown) = &runner.shown {
                ui.weak(summary(shown));
            }
        }
        status => problem(ui, status),
    }
}

fn progress(ui: &mut Ui, runner: &Runner) {
    let fraction = runner.shown.as_ref().map_or(0.0, |s| s.snapshot.fraction);
    ui.add(
        ProgressBar::new(fraction as f32)
            .fill(theme::accent(ui, Accent::Results))
            .animate(true)
            .show_percentage(),
    );
}

fn problem(ui: &mut Ui, status: &Status) {
    match status {
        Status::Running | Status::Finished | Status::Stopped => {}
        Status::Failed(msg) => error_label(ui, msg),
        Status::Invalid(ScenarioError::NoQueries) => {
            error_label(ui, "Füge eine Abfrage mit mindestens einem Baustein hinzu.")
        }
        // Expression errors are shown where the expression is edited.
        Status::Invalid(ScenarioError::Expr {
            site: ExprSite::MeWhere,
            ..
        }) => error_label(ui, "Die weitere Bedingung enthält einen Fehler."),
        Status::Invalid(ScenarioError::Expr { .. }) => {
            error_label(ui, "Eine Abfrage enthält einen Fehler.")
        }
        Status::Invalid(e) => error_label(ui, &e.to_string()),
    }
}

fn error_label(ui: &mut Ui, msg: &str) {
    ui.label(RichText::new(msg).color(ui.visuals().error_fg_color));
}

fn summary(shown: &Shown) -> String {
    let o = &shown.snapshot.outcome;
    let mut text = format!(
        "{} von {} gemischt · {} s",
        format::int(o.accepted),
        format::int(shown.settings.iterations),
        format::decimal(shown.snapshot.elapsed.as_secs_f64(), 2)
    );
    if o.attempts != o.accepted {
        text += &format!(
            " · davon passend {}",
            format::percent(ratio(o.accepted, o.attempts))
        );
    }
    text
}

fn result_card(
    ui: &mut Ui,
    index: usize,
    result: &QueryResult,
    examples: &[Deal],
    shown: &Shown,
    me: Position,
) {
    let e = &result.estimate;
    let accent = theme::accent(ui, Accent::Results);
    theme::card(ui, accent, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&result.name).strong());
            // One row high: a bare right-to-left layout would take the rest of the card.
            let row = egui::vec2(ui.available_width(), 0.0);
            let layout = egui::Layout::right_to_left(egui::Align::Center);
            ui.allocate_ui_with_layout(row, layout, |ui| {
                ui.label(RichText::new(format::percent(e.p)).size(20.0).strong());
            });
        });
        ui.weak(format!(
            "± {} (95 %) · {} Treffer",
            format::decimal(format::PERCENT * e.margin(), 2),
            format::int(e.hits)
        ));
        if !examples.is_empty() {
            egui::CollapsingHeader::new("Beispiele")
                .id_salt(("examples", index))
                .show(ui, |ui| {
                    for deal in examples {
                        deal_view(ui, deal, shown, me);
                        ui.separator();
                    }
                });
        }
    });
}

fn deal_view(ui: &mut Ui, deal: &Deal, shown: &Shown, me: Position) {
    for position in Position::ALL {
        let seat = position.seat(me);
        ui.horizontal(|ui| {
            let mut label = position_label(position, me);
            if deal.partner == Some(seat) {
                label += " · Partner";
            }
            ui.add_sized([EXAMPLE_LABEL_WIDTH, 0.0], egui::Label::new(label));
            for card in shown.scenario.game.sorted(deal.hands[seat.index()]) {
                paint::card(ui, card, CardLook::Normal, EXAMPLE_CARD_SCALE);
            }
        });
    }
}
