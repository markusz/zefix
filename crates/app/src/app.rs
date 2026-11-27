//! App state and the frame loop. How the screen is laid out lives in `layout`.

use crate::layout::{Layout, desktop, phone};
use crate::model::Model;
use crate::runner::{RunSettings, Runner, Status};
use crate::theme;
use crate::view::queries::{self, QueryError};
use crate::view::share::ShareState;
use crate::view::tiles::EditState;
use crate::view::{results, seats, share};
use eframe::egui::{self, Ui};
use zefix_core::game::Game;
use zefix_core::scenario::{ExprSite, ScenarioError};

/// Enough for about ±0,1 percentage points; more is one click away in the top bar.
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_ITERATIONS: u64 = 1_000_000;
/// The browser simulates on one core, so it starts smaller (about ±0,3 points).
#[cfg(target_arch = "wasm32")]
const DEFAULT_ITERATIONS: u64 = 100_000;

pub struct ZefixApp {
    pub(crate) model: Model,
    pub(crate) settings: RunSettings,
    pub(crate) edit: EditState,
    pub(crate) runner: Runner,
    pub(crate) share: ShareState,
    pub(crate) tab: phone::Tab,
    /// The model last mirrored into the address bar.
    #[cfg(target_arch = "wasm32")]
    in_location: Option<Model>,
}

impl ZefixApp {
    pub fn new(ctx: &egui::Context) -> ZefixApp {
        theme::install(ctx);
        egui_extras::install_image_loaders(ctx);
        let app = ZefixApp {
            model: Model::default(),
            settings: RunSettings {
                iterations: DEFAULT_ITERATIONS,
            },
            edit: EditState::default(),
            runner: Runner::new(),
            share: ShareState::default(),
            tab: phone::Tab::default(),
            #[cfg(target_arch = "wasm32")]
            in_location: None,
        };
        #[cfg(target_arch = "wasm32")]
        let app = app.restored_from_location();
        app
    }

    /// Opens what a shared link carries.
    #[cfg(target_arch = "wasm32")]
    fn restored_from_location(mut self) -> ZefixApp {
        match crate::share::location::read() {
            Some(Ok(model)) => self.model = model,
            Some(Err(e)) => self.share.show_error(e),
            None => {}
        }
        self
    }

    fn filter_error(&self) -> Option<String> {
        match &self.runner.status {
            Status::Invalid(ScenarioError::Expr {
                site: ExprSite::MeWhere,
                src,
                error,
            }) => Some(error.render(src)),
            _ => None,
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
        crate::keyboard::begin_frame(ui.ctx());
        let layout = Layout::for_width(ui.available_width());
        theme::set_density(ui, layout.density());
        match layout {
            Layout::Desktop => desktop::top_bar(self, ui),
            Layout::Phone => phone::top_bar(self, ui),
        }
        let Some(game) = self.model.game.game() else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("Ungültiges Spiel.");
            });
            return;
        };
        match layout {
            Layout::Desktop => desktop::body(self, ui, &game),
            Layout::Phone => phone::body(self, ui, &game),
        }
        share::show(ui.ctx(), &mut self.share, &mut self.model, &game);
        // After the UI so edits from this frame start a new run immediately.
        self.runner
            .sync(self.model.spec(&game), self.settings, ui.ctx());
        #[cfg(target_arch = "wasm32")]
        if self.in_location.as_ref() != Some(&self.model) {
            crate::share::location::write(&self.model);
            self.in_location = Some(self.model.clone());
        }
    }
}

/// The sections both layouts place, wired to the app state.
impl ZefixApp {
    pub(crate) fn seats(&mut self, ui: &mut Ui, game: &Game) {
        let error = self.filter_error();
        seats::show(ui, &mut self.model, &mut self.edit, game, error.as_deref());
    }

    pub(crate) fn queries(&mut self, ui: &mut Ui, game: &Game, inline: Option<&results::Inline>) {
        let error = self.query_error();
        queries::show(
            ui,
            &mut self.model,
            &mut self.edit,
            game,
            error.as_ref(),
            inline,
        );
    }

    pub(crate) fn results(&mut self, ui: &mut Ui) {
        results::show(ui, &mut self.runner, self.model.my_position);
    }
}

/// Renders the window for visual review. Needs a GPU, so it only runs on request:
/// `cargo test -p zefix-app -- --ignored`; images land in `target/ui-snapshots/`.
#[cfg(test)]
mod snapshots {
    use super::*;
    use crate::layout::controls::PRECISIONS;
    use crate::layout::phone::Tab;
    use crate::model::{CardSet, CountOp, GameChoice, Position, QueryModel, Scope, Tile};
    use crate::view::tiles::Selection;
    use crate::view::tiles::Target;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use std::path::PathBuf;
    use std::time::Duration;
    use zefix_core::cards::{Card, Suit};
    use zefix_core::game::Seat;

    const SIZE: [f32; 2] = [1440.0, 960.0];
    /// Narrower than common phones (320 px) to leave headroom for zoom and large text.
    const PHONE: [f32; 2] = [250.0, 640.0];
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
                Tile::Card(Card::new(Suit::Schellen, zefix_core::cards::Rank::Ober)),
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

    fn render_phone(name: &str, setup: impl FnOnce(&mut ZefixApp)) {
        let setup = |app: &mut ZefixApp| {
            // Short runs, so the inline results are in before the picture is taken.
            app.settings.iterations = PRECISIONS[0].iterations;
            setup(app);
        };
        render_sized(name, PHONE, egui::Theme::Light, setup, |_| {});
    }

    /// Like `render`, with `interact` run once the first frame is laid out (e.g. a click).
    fn render_after(
        name: &str,
        theme: egui::Theme,
        setup: impl FnOnce(&mut ZefixApp),
        interact: impl FnOnce(&mut Harness<'_, ZefixApp>),
    ) {
        render_sized(name, SIZE, theme, setup, interact);
    }

    fn render_sized(
        name: &str,
        size: [f32; 2],
        theme: egui::Theme,
        setup: impl FnOnce(&mut ZefixApp),
        interact: impl FnOnce(&mut Harness<'_, ZefixApp>),
    ) {
        let mut harness = Harness::builder()
            .with_size(size)
            // Screenshots only; unlabeled combo boxes are a separate accessibility task.
            .with_accessibility_check(false)
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
        render("share", egui::Theme::Dark, |app| {
            app.model = tout();
            app.model.filter_text = "left.frei(herz) or right.frei(herz)".into();
            app.share.open = true;
        });
        render_phone("phone", |_| {});
        render_phone("phone_tout", |app| app.model = tout());
        render_phone("phone_picker", |app| {
            app.model = tout();
            app.edit.picker = Some(Target::Seat(Seat::Left));
        });
        render_phone("phone_queries", |app| {
            app.model = tout();
            app.tab = Tab::Queries;
        });
        render_phone("phone_results", |app| {
            app.tab = Tab::Queries;
            app.model.queries.push(QueryModel::visual(
                "Partner hat ≥ 3 Trumpf",
                Scope::Partner,
                vec![Tile::Count {
                    set: CardSet::Trumpf,
                    op: CountOp::AtLeast(3),
                }],
            ));
        });
        render("query_error", egui::Theme::Dark, |app| {
            if let Some(query) = app.model.queries.first_mut() {
                query.mode = crate::model::QueryMode::Text;
                query.text = "any(p in others, p.count(trumf) > 3)".into();
            }
        });
    }
}
