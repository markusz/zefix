//! Settings controls both layouts place in their top bars.

use crate::model::{GameChoice, GameSetup, Model, Position};
use crate::runner::RunSettings;
use eframe::egui::{self, ComboBox, RichText, Ui, Vec2};

const LOGO_SIZE: f32 = 26.0;
const BRAND_SIZE: f32 = 18.0;
const PRECISION_WIDTH: f32 = 110.0;

/// A run length named by what it buys: how precise the numbers get.
pub struct Precision {
    pub name: &'static str,
    pub iterations: u64,
}

pub const PRECISIONS: [Precision; 4] = [
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

pub fn brand(ui: &mut Ui) {
    ui.add(
        egui::Image::new(egui::include_image!("../../assets/logo/zefix.svg"))
            .fit_to_exact_size(Vec2::splat(LOGO_SIZE)),
    );
    ui.label(RichText::new("zefix!").strong().size(BRAND_SIZE));
}

/// The game dropdown; each layout adds its own suit picker after it.
pub fn game_choice(ui: &mut Ui, setup: &mut GameSetup) {
    ComboBox::from_id_salt("game")
        .selected_text(setup.choice.label())
        .show_ui(ui, |ui| {
            for choice in GameChoice::ALL {
                ui.selectable_value(&mut setup.choice, choice, choice.label());
            }
        });
    setup.normalize();
}

pub fn position_picker(ui: &mut Ui, model: &mut Model) {
    ui.label("Position");
    for position in Position::ALL {
        let label = format!("{}.", position.number());
        ui.selectable_value(&mut model.my_position, position, label)
            .on_hover_text("Position in der Spielreihenfolge; 1. = Vorhand");
    }
}

pub fn run_settings(ui: &mut Ui, settings: &mut RunSettings) {
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
