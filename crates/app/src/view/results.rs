//! Live results: one probability bar with 95 % interval per query, plus example deals.

use super::seats::position_label;
use crate::format;
use crate::model::Position;
use crate::paint::{self, CardLook};
use crate::runner::{Runner, Shown, Status};
use crate::theme::{self, Accent};
use eframe::egui::{self, CornerRadius, ProgressBar, RichText, Sense, Stroke, Ui, vec2};
use schafsim_core::lang::eval::Deal;
use schafsim_core::scenario::ScenarioError;
use schafsim_core::stats::{self, QueryResult, ratio};

const BAR_HEIGHT: f32 = 14.0;
const EXAMPLE_CARD_SCALE: f32 = 0.6;
const EXAMPLE_LABEL_WIDTH: f32 = 150.0;
/// Opacity of results that belong to an older model.
const STALE_OPACITY: f32 = 0.45;

pub fn show(ui: &mut Ui, runner: &mut Runner, me: Position) {
    let accent = theme::accent(ui, Accent::Results);
    theme::section_header(ui, "Ergebnis", "", accent);
    ui.horizontal(|ui| {
        if runner.is_running() {
            if theme::action(ui, "Stopp", accent).clicked() {
                runner.stop();
            }
        } else if theme::action(ui, "Neu berechnen", accent).clicked() {
            runner.restart();
        }
        status(ui, runner, accent);
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

fn status(ui: &mut Ui, runner: &Runner, accent: egui::Color32) {
    match &runner.status {
        Status::Running => {
            let fraction = runner.shown.as_ref().map_or(0.0, |s| s.snapshot.fraction);
            ui.add(
                ProgressBar::new(fraction as f32)
                    .fill(accent)
                    .animate(true)
                    .show_percentage(),
            );
        }
        Status::Finished | Status::Stopped => {
            if let Some(shown) = &runner.shown {
                ui.weak(summary(shown));
            }
        }
        Status::Failed(msg) => error_label(ui, msg),
        Status::Invalid(ScenarioError::NoQueries) => {
            error_label(ui, "Füge eine Abfrage mit mindestens einem Baustein hinzu.")
        }
        // Query errors are shown at the query itself.
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
        "{} von {} Deals · {} s",
        format::int(o.accepted),
        format::int(shown.settings.iterations),
        format::decimal(shown.snapshot.elapsed.as_secs_f64(), 2)
    );
    if o.attempts != o.accepted {
        text += &format!(
            " · Akzeptanz {}",
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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format::percent(e.p)).size(20.0).strong());
            });
        });
        probability_bar(ui, e.p, e.ci95.lo, e.ci95.hi, accent);
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

fn probability_bar(ui: &mut Ui, p: f64, lo: f64, hi: f64, accent: egui::Color32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, BAR_HEIGHT), Sense::hover());
    let painter = ui.painter();
    let visuals = ui.visuals();
    let x = |v: f64| rect.left() + rect.width() * v.clamp(0.0, 1.0) as f32;
    painter.rect_filled(rect, CornerRadius::same(3), visuals.extreme_bg_color);
    let mut filled = rect;
    filled.set_right(x(p));
    painter.rect_filled(filled, CornerRadius::same(3), accent);
    let whisker = Stroke::new(1.5, visuals.strong_text_color());
    let (top, bottom) = (rect.top() + 2.0, rect.bottom() - 2.0);
    painter.line_segment(
        [
            (x(lo), rect.center().y).into(),
            (x(hi), rect.center().y).into(),
        ],
        whisker,
    );
    for v in [lo, hi] {
        painter.line_segment([(x(v), top).into(), (x(v), bottom).into()], whisker);
    }
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
