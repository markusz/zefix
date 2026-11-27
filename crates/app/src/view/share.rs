//! The "Teilen" window: copy a link or the TOML, or bring a scenario in from either.

use crate::model::Model;
use crate::share;
use eframe::egui::{self, RichText, TextEdit, Ui};
use zefix_core::game::Game;

const WINDOW_WIDTH: f32 = 560.0;
const IMPORT_ROWS: usize = 6;
const WINDOW_OFFSET: egui::Vec2 = egui::vec2(-16.0, 56.0);

#[derive(Default)]
pub struct ShareState {
    pub open: bool,
    import_text: String,
    message: Option<Message>,
}

enum Message {
    Copied(&'static str),
    Imported { dropped: Vec<String> },
    Error(String),
}

impl ShareState {
    /// Opens the window with an error, e.g. for a broken link in the address bar.
    #[cfg(target_arch = "wasm32")]
    pub fn show_error(&mut self, error: String) {
        self.open = true;
        self.message = Some(Message::Error(error));
    }
}

pub fn show(ctx: &egui::Context, state: &mut ShareState, model: &mut Model, game: &Game) {
    let mut open = state.open;
    egui::Window::new("Teilen")
        .open(&mut open)
        .collapsible(false)
        .default_width(WINDOW_WIDTH)
        // Below the top bar, by the button that opened it.
        .pivot(egui::Align2::RIGHT_TOP)
        .default_pos(ctx.content_rect().right_top() + WINDOW_OFFSET)
        .show(ctx, |ui| {
            export(ui, state, model, game);
            ui.separator();
            import(ui, state, model);
            message(ui, state.message.as_ref());
        });
    state.open = open;
    if !open {
        state.message = None;
    }
}

fn export(ui: &mut Ui, state: &mut ShareState, model: &Model, game: &Game) {
    ui.label("Der Link stellt alles genau so wieder her, wie es gerade ist.");
    ui.horizontal(|ui| {
        if ui.button("Link kopieren").clicked() {
            ui.ctx().copy_text(share::link(model));
            state.message = Some(Message::Copied("Link"));
        }
        if ui.button("TOML kopieren").clicked() {
            ui.ctx().copy_text(share::to_toml(model, game));
            state.message = Some(Message::Copied("TOML"));
        }
    });
    ui.weak("TOML ist das Format der Kommandozeile `zefix` und des MCP-Servers.");
    egui::CollapsingHeader::new("TOML ansehen")
        .id_salt("share_toml")
        .show(ui, |ui| {
            let mut text = share::to_toml(model, game);
            ui.add(
                TextEdit::multiline(&mut text)
                    .code_editor()
                    .interactive(false)
                    .desired_width(f32::INFINITY),
            );
        });
}

fn import(ui: &mut Ui, state: &mut ShareState, model: &mut Model) {
    ui.label("Importieren: Link oder TOML einfügen.");
    let field = ui.add(
        TextEdit::multiline(&mut state.import_text)
            .code_editor()
            .desired_rows(IMPORT_ROWS)
            .desired_width(f32::INFINITY),
    );
    crate::keyboard::text_field(&field);
    let ready = !state.import_text.trim().is_empty();
    if ui
        .add_enabled(ready, egui::Button::new("Übernehmen"))
        .on_hover_text("Ersetzt das aktuelle Szenario und alle Abfragen.")
        .clicked()
    {
        state.message = Some(match share::import(&state.import_text) {
            Ok(imported) => {
                *model = imported.model;
                state.import_text.clear();
                Message::Imported {
                    dropped: imported.dropped,
                }
            }
            Err(e) => Message::Error(e),
        });
    }
}

fn message(ui: &mut Ui, message: Option<&Message>) {
    let Some(message) = message else { return };
    match message {
        Message::Copied(what) => {
            ui.label(format!("{what} kopiert."));
        }
        Message::Imported { dropped } if dropped.is_empty() => {
            ui.label("Übernommen.");
        }
        Message::Imported { dropped } => {
            ui.label(format!("Übernommen, ohne: {}.", dropped.join("; ")));
        }
        Message::Error(e) => {
            ui.label(
                RichText::new(e)
                    .monospace()
                    .color(ui.visuals().error_fg_color),
            );
        }
    }
}
