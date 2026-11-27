//! Window layout: settings on top, seats on the left, queries and live results on the right;
//! on narrow screens the three sections become tabs.

use crate::model::{GameChoice, Model, Position};
use crate::paint;
use crate::runner::{RunSettings, Runner, Status};
use crate::theme::{self, Density};
use crate::view::queries::{self, QueryError};
use crate::view::share::ShareState;
use crate::view::tiles::EditState;
use crate::view::{results, seats, share};
use eframe::egui::{self, ComboBox, RichText, ScrollArea, Ui, Vec2};
use zefix_core::cards::Suit;
use zefix_core::game::Game;
use zefix_core::scenario::{ExprSite, ScenarioError};

const SEAT_PANEL_WIDTH: f32 = 760.0;
/// Below this width (phones) the sections become tabs instead of columns.
const NARROW_WIDTH: f32 = 700.0;
const LOGO_SIZE: f32 = 26.0;
const PRECISION_WIDTH: f32 = 110.0;
const SUIT_BUTTON_EXTRA: f32 = 6.0;
const SUIT_BUTTON_CORNER: u8 = 6;
/// Symbol radius as a share of the button height.
const SUIT_ICON_SHARE: f32 = 0.36;
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
    share: ShareState,
    tab: Tab,
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
            tab: Tab::default(),
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
        let layout = Layout::for_width(ui.available_width());
        theme::set_density(ui, layout.density());
        egui::Panel::top("settings")
            .frame(theme::top_bar_frame(ui))
            .show(ui, |ui| {
                let bar = TopBar {
                    model: &mut self.model,
                    settings: &mut self.settings,
                    share_open: &mut self.share.open,
                };
                match layout {
                    Layout::Columns => bar.show_in_line(ui),
                    Layout::Tabs => bar.show_stacked(ui),
                }
            });
        let Some(game) = self.model.game.game() else {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("Ungültiges Spiel.");
            });
            return;
        };
        match layout {
            Layout::Columns => self.side_by_side(ui, &game),
            Layout::Tabs => self.tabbed(ui, &game),
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

impl ZefixApp {
    fn side_by_side(&mut self, ui: &mut Ui, game: &Game) {
        egui::Panel::left("seats")
            .resizable(true)
            .default_size(SEAT_PANEL_WIDTH)
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| self.seats(ui, game));
            });
        egui::CentralPanel::default().show(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                self.queries(ui, game, None);
                ui.separator();
                self.results(ui);
            });
        });
    }

    /// Phones: one section at a time, each at full width.
    fn tabbed(&mut self, ui: &mut Ui, game: &Game) {
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for tab in Tab::ALL {
                    ui.selectable_value(&mut self.tab, tab, tab.label());
                }
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ScrollArea::vertical()
                .id_salt(self.tab)
                .show(ui, |ui| match self.tab {
                    Tab::Seats => self.seats(ui, game),
                    Tab::Queries => {
                        // Results sit inside each query; run length and progress go below.
                        let inline = results::Inline::of(&self.runner);
                        self.queries(ui, game, inline.as_ref());
                        ui.separator();
                        ui.horizontal_wrapped(|ui| run_settings(ui, &mut self.settings));
                        results::status_line(ui, &self.runner);
                    }
                });
        });
    }

    fn seats(&mut self, ui: &mut Ui, game: &Game) {
        let error = self.filter_error();
        seats::show(ui, &mut self.model, &mut self.edit, game, error.as_deref());
    }

    fn queries(&mut self, ui: &mut Ui, game: &Game, inline: Option<&results::Inline>) {
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

    fn results(&mut self, ui: &mut Ui) {
        results::show(ui, &mut self.runner, self.model.my_position);
    }
}

/// The one place that decides between desktop and phone layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    /// Seats beside queries and results.
    Columns,
    /// Phones: one section at a time.
    Tabs,
}

impl Layout {
    fn for_width(width: f32) -> Layout {
        if width < NARROW_WIDTH {
            Layout::Tabs
        } else {
            Layout::Columns
        }
    }

    fn density(self) -> Density {
        match self {
            Layout::Columns => Density::Regular,
            Layout::Tabs => Density::Compact,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Tab {
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

struct TopBar<'a> {
    model: &'a mut Model,
    settings: &'a mut RunSettings,
    share_open: &'a mut bool,
}

impl TopBar<'_> {
    fn show_in_line(self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            brand(ui);
            ui.separator();
            game_picker(ui, self.model, SuitPicker::Buttons);
            ui.separator();
            position_picker(ui, self.model);
            ui.separator();
            run_settings(ui, self.settings);
            ui.separator();
            share_button(ui, self.share_open);
        });
    }

    /// One group per row: wrapped separators would stretch to the full bar height.
    /// No share button: the browser's own share sheet sends the address, which holds the state.
    fn show_stacked(self, ui: &mut Ui) {
        ui.horizontal(brand);
        ui.horizontal_wrapped(|ui| game_picker(ui, self.model, SuitPicker::Icons));
        ui.horizontal_wrapped(|ui| position_picker(ui, self.model));
    }
}

fn brand(ui: &mut Ui) {
    ui.add(
        egui::Image::new(egui::include_image!("../assets/logo/zefix.svg"))
            .fit_to_exact_size(Vec2::splat(LOGO_SIZE)),
    );
    ui.label(RichText::new("zefix!").strong().size(18.0));
}

fn position_picker(ui: &mut Ui, model: &mut Model) {
    ui.label("Position");
    for position in Position::ALL {
        let label = format!("{}.", position.number());
        ui.selectable_value(&mut model.my_position, position, label)
            .on_hover_text("Position in der Spielreihenfolge; 1. = Vorhand");
    }
}

fn share_button(ui: &mut Ui, open: &mut bool) {
    if ui
        .button("Teilen")
        .on_hover_text("Link oder TOML kopieren, Szenario importieren")
        .clicked()
    {
        *open = true;
    }
}

/// Named suit buttons need a wide bar; bare symbols fit next to the game on a phone.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SuitPicker {
    Buttons,
    Icons,
}

fn game_picker(ui: &mut Ui, model: &mut Model, picker: SuitPicker) {
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
    if picker == SuitPicker::Icons {
        for &suit in suits {
            let hint = format!("{}: {}", setup.choice.suit_label(), suit.display_name());
            if suit_button(ui, suit, setup.suit == suit)
                .on_hover_text(hint)
                .clicked()
            {
                setup.suit = suit;
            }
        }
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

/// A square toggle showing only the suit symbol.
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
/// `cargo test -p zefix-app -- --ignored`; images land in `target/ui-snapshots/`.
#[cfg(test)]
mod snapshots {
    use super::*;
    use crate::model::{CardSet, CountOp, QueryModel, Scope, Tile};
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
