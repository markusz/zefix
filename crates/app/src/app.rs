//! Window layout: settings on top, seats on the left, queries and live results on the right.

use crate::model::{GameChoice, Model, Position};
use crate::paint;
use crate::runner::{RunSettings, Runner, Status};
use crate::theme;
use crate::view::queries::{self, QueryError};
use crate::view::tiles::EditState;
use crate::view::{results, seats};
use eframe::egui::{self, ComboBox, RichText, ScrollArea, Ui, Vec2};
use schafsim_core::scenario::{ExprSite, ScenarioError};

const SEAT_PANEL_WIDTH: f32 = 760.0;
const LOGO_SIZE: f32 = 26.0;
const PRECISION_WIDTH: f32 = 110.0;
/// A run length named by what it buys: how precise the numbers get.
struct Precision {
    name: &'static str,
    iterations: u64,
}

const PRECISIONS: [Precision; 4] = [
    Precision {
        name: "Sehr schnell",
        iterations: 10_000,
    },
    Precision {
        name: "Schnell",
        iterations: 100_000,
    },
    Precision {
        name: "Normal",
        iterations: 1_000_000,
    },
    Precision {
        name: "Genau",
        iterations: 10_000_000,
    },
];
/// Enough for about ±0,1 percentage points; more is one click away in the top bar.
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_ITERATIONS: u64 = 1_000_000;
/// The browser simulates on one core, so it starts smaller (about ±0,3 points).
#[cfg(target_arch = "wasm32")]
const DEFAULT_ITERATIONS: u64 = 100_000;

pub struct ZefixApp {
    model: Model,
    settings: RunSettings,
    edit: EditState,
    runner: Runner,
}

impl ZefixApp {
    pub fn new(ctx: &egui::Context) -> ZefixApp {
        theme::install(ctx);
        egui_extras::install_image_loaders(ctx);
        ZefixApp {
            model: Model::default(),
            settings: RunSettings {
                iterations: DEFAULT_ITERATIONS,
            },
            edit: EditState::default(),
            runner: Runner::new(),
        }
    }

    fn query_error(&self) -> Option<QueryError> {
        match &self.runner.status {
            Status::Invalid(ScenarioError::Expr {
                site: ExprSite::Query { name },
                src,
                error,
            }) => Some(QueryError {
                name: name.clone(),
                rendered: error.render(src),
            }),
            _ => None,
        }
    }
}

impl eframe::App for ZefixApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("settings")
            .frame(theme::top_bar_frame(ui))
            .show(ui, |ui| {
                top_bar(ui, &mut self.model, &mut self.settings);
            });
        let Some(game) = self.model.game.game() else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("Ungültiges Spiel.");
            });
            return;
        };
        egui::Panel::left("seats")
            .resizable(true)
            .default_size(SEAT_PANEL_WIDTH)
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    seats::show(ui, &mut self.model, &mut self.edit, &game);
                });
            });
        let error = self.query_error();
        egui::CentralPanel::default().show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                queries::show(ui, &mut self.model, &mut self.edit, &game, error.as_ref());
                ui.separator();
                results::show(ui, &mut self.runner, self.model.my_position);
            });
        });
        // After the UI so edits from this frame start a new run immediately.
        self.runner
            .sync(self.model.spec(&game), self.settings, ui.ctx());
    }
}

fn top_bar(ui: &mut Ui, model: &mut Model, settings: &mut RunSettings) {
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::Image::new(egui::include_image!("../assets/logo/zefix.svg"))
                .fit_to_exact_size(Vec2::splat(LOGO_SIZE)),
        );
        ui.label(RichText::new("zefix!").strong().size(18.0));
        ui.separator();
        game_picker(ui, model);
        ui.separator();
        ui.label("Ich sitze auf");
        for position in Position::ALL {
            let label = format!("{}.", position.number());
            ui.selectable_value(&mut model.my_position, position, label)
                .on_hover_text("Position in der Spielreihenfolge; 1. = Vorhand");
        }
        ui.separator();
        run_settings(ui, settings);
    });
}

fn game_picker(ui: &mut Ui, model: &mut Model) {
    let setup = &mut model.game;
    ComboBox::from_id_salt("game")
        .selected_text(setup.choice.label())
        .show_ui(ui, |ui| {
            for choice in GameChoice::ALL {
                ui.selectable_value(&mut setup.choice, choice, choice.label());
            }
        });
    setup.normalize();
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

fn run_settings(ui: &mut Ui, settings: &mut RunSettings) {
    ui.label("Genauigkeit");
    let current = PRECISIONS
        .iter()
        .find(|p| p.iterations == settings.iterations)
        .map_or("?", |p| p.name);
    ComboBox::from_id_salt("precision")
        .selected_text(current)
        .width(PRECISION_WIDTH)
        .show_ui(ui, |ui| {
            for p in PRECISIONS {
                ui.selectable_value(&mut settings.iterations, p.iterations, p.name);
            }
        });
}

/// Renders the window for visual review. Needs a GPU, so it only runs on request:
/// `cargo test -p schafsim-app -- --ignored`; images land in `target/ui-snapshots/`.
#[cfg(test)]
mod snapshots {
    use super::*;
    use crate::model::{CardSet, CountOp, QueryModel, Scope, Tile};
    use crate::view::tiles::Selection;
    use crate::view::tiles::Target;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use schafsim_core::cards::{Card, Suit};
    use schafsim_core::game::Seat;
    use std::path::PathBuf;
    use std::time::Duration;

    const SIZE: [f32; 2] = [1440.0, 960.0];
    const STEPS: usize = 40;
    const STEP_PAUSE: Duration = Duration::from_millis(25);

    fn tout() -> Model {
        let mut model = Model {
            my_position: Position::Second,
            ..Model::default()
        };
        model.game.choice = GameChoice::Solo;
        model.game.suit = Suit::Eichel;
        model.seat_mut(Seat::Me).tiles = ["EO", "GO", "HO", "EU", "GU", "EA", "E10", "GA"]
            .iter()
            .filter_map(|c| Card::parse(c).map(Tile::Card))
            .collect();
        model.seat_mut(Seat::Left).tiles = vec![Tile::Count {
            set: CardSet::Suit(Suit::Gras),
            op: CountOp::Zero,
        }];
        model.seat_mut(Seat::Right).tiles = vec![
            Tile::Count {
                set: CardSet::Farbe,
                op: CountOp::AtMost(2),
            },
            Tile::Count {
                set: CardSet::Sau,
                op: CountOp::AtLeast(1),
            },
            Tile::Count {
                set: CardSet::Suit(Suit::Herz),
                op: CountOp::Exactly(2),
            },
        ];
        model.seat_mut(Seat::Across).tiles = vec![Tile::Count {
            set: CardSet::Ober,
            op: CountOp::Between { min: 1, max: 3 },
        }];
        model.queries.push(QueryModel::visual(
            "Ein Gegner hat SO + ≥ 3 Trumpf",
            Scope::AnyOther,
            vec![
                Tile::Card(Card::new(Suit::Schellen, schafsim_core::cards::Rank::Ober)),
                Tile::Count {
                    set: CardSet::Trumpf,
                    op: CountOp::AtLeast(3),
                },
            ],
        ));
        model
    }

    fn render(name: &str, theme: egui::Theme, setup: impl FnOnce(&mut ZefixApp)) {
        render_after(name, theme, setup, |_| {});
    }

    /// Like `render`, with `interact` run once the first frame is laid out (e.g. a click).
    fn render_after(
        name: &str,
        theme: egui::Theme,
        setup: impl FnOnce(&mut ZefixApp),
        interact: impl FnOnce(&mut Harness<'_, ZefixApp>),
    ) {
        let mut harness = Harness::builder()
            .with_size(SIZE)
            .with_theme(theme)
            .wgpu()
            .build_eframe(|cc| ZefixApp::new(&cc.egui_ctx));
        setup(harness.state_mut());
        harness.step();
        interact(&mut harness);
        for _ in 0..STEPS {
            harness.step();
            std::thread::sleep(STEP_PAUSE);
        }
        let image = harness.render().unwrap_or_else(|e| panic!("{e}"));
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/ui-snapshots");
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
        image
            .save(dir.join(format!("{name}.png")))
            .unwrap_or_else(|e| panic!("{e}"));
    }

    #[test]
    #[ignore]
    fn main_window() {
        render("default", egui::Theme::Dark, |_| {});
        render("tout_light", egui::Theme::Light, |app| {
            app.model = tout();
            app.edit.selected = Some(Selection {
                target: Target::Seat(Seat::Across),
                index: 0,
            });
        });
        render("tout", egui::Theme::Dark, |app| {
            app.model = tout();
            app.edit.picker = Some(Target::Seat(Seat::Me));
        });
        render_after(
            "precision_open",
            egui::Theme::Dark,
            |_| {},
            |h| h.get_by_value("Normal").click(),
        );
        render("query_error", egui::Theme::Dark, |app| {
            if let Some(query) = app.model.queries.first_mut() {
                query.mode = crate::model::QueryMode::Text;
                query.text = "any(p in others, p.count(trumf) > 3)".into();
            }
        });
    }
}
