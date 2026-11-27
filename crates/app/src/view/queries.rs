//! Query editor: each query is either built from tiles plus a "who" selector, or typed as text.

use super::seats::position_label;
use super::tiles::{self, EditState, Target, TileRow};
use crate::model::{Model, OTHER_PLAYERS, Position, QueryMode, QueryModel, Scope};
use crate::theme::{self, Accent};
use eframe::egui::{Color32, ComboBox, DragValue, RichText, TextEdit, Ui};
use zefix_core::cards::Cards;
use zefix_core::game::Game;

const NAME_WIDTH: f32 = 260.0;

/// A compile error for one query, already rendered with a caret under the bad spot.
pub struct QueryError {
    pub name: String,
    pub rendered: String,
}

pub fn show(
    ui: &mut Ui,
    model: &mut Model,
    edit: &mut EditState,
    game: &Game,
    error: Option<&QueryError>,
) {
    let accent = theme::accent(ui, Accent::Queries);
    theme::section_header(ui, "Abfragen", "Wie wahrscheinlich ist …", accent);
    let me = model.my_position;
    let mut removed = None;
    for (index, query) in model.queries.iter_mut().enumerate() {
        let error = error.filter(|e| e.name == query.name);
        let row = QueryRow {
            index,
            me,
            error,
            accent,
        };
        if query_frame(ui, query, edit, game, row) == RowAction::Remove {
            removed = Some(index);
        }
        ui.add_space(6.0);
    }
    if theme::action(ui, "+ Abfrage", accent).clicked() {
        model.queries.push(QueryModel::visual(
            &format!("Abfrage {}", model.queries.len() + 1),
            Scope::AnyOther,
            Vec::new(),
        ));
    }
    if let Some(index) = removed {
        model.queries.remove(index);
        // Indices of later queries shift, so drop all query selections.
        for i in index..=model.queries.len() {
            edit.release(Target::Query(i));
        }
    }
}

struct QueryRow<'a> {
    index: usize,
    me: Position,
    error: Option<&'a QueryError>,
    accent: Color32,
}

#[derive(PartialEq, Eq)]
enum RowAction {
    Keep,
    Remove,
}

fn query_frame(
    ui: &mut Ui,
    query: &mut QueryModel,
    edit: &mut EditState,
    game: &Game,
    row: QueryRow<'_>,
) -> RowAction {
    theme::card(ui, row.accent, |ui| {
        let action = title_bar(ui, query, game, row.index);
        match query.mode {
            QueryMode::Visual => visual(ui, query, edit, game, &row),
            QueryMode::Text => {
                ui.add(
                    TextEdit::multiline(&mut query.text)
                        .code_editor()
                        .desired_rows(2)
                        .desired_width(f32::INFINITY),
                );
            }
        }
        if let Some(error) = row.error {
            theme::inset(ui, |ui| {
                ui.label(
                    RichText::new(&error.rendered)
                        .monospace()
                        .color(ui.visuals().error_fg_color),
                );
            });
        }
        action
    })
}

fn title_bar(ui: &mut Ui, query: &mut QueryModel, game: &Game, index: usize) -> RowAction {
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut query.name).desired_width(NAME_WIDTH));
        let before = query.mode;
        ui.selectable_value(&mut query.mode, QueryMode::Visual, "Bausteine");
        ui.selectable_value(&mut query.mode, QueryMode::Text, "Text");
        // Switching to text starts from the generated expression, so nothing is lost.
        if before == QueryMode::Visual
            && query.mode == QueryMode::Text
            && query.text.trim().is_empty()
        {
            query.text = query.visual_expr(game).unwrap_or_default();
        }
        let remove = theme::danger(ui, "Löschen")
            .on_hover_text(format!("Abfrage {} löschen", index + 1))
            .clicked();
        if remove {
            RowAction::Remove
        } else {
            RowAction::Keep
        }
    })
    .inner
}

fn visual(
    ui: &mut Ui,
    query: &mut QueryModel,
    edit: &mut EditState,
    game: &Game,
    row: &QueryRow<'_>,
) {
    ui.horizontal(|ui| {
        ui.label("Wer:");
        scope_picker(ui, &mut query.scope, game, row);
    });
    let tiles_row = TileRow {
        target: Target::Query(row.index),
        tiles: &mut query.tiles,
        blocked: Cards::EMPTY,
        accent: row.accent,
    };
    tiles::show(ui, game, edit, tiles_row);
    theme::inset(ui, |ui| {
        ui.set_width(ui.available_width());
        match query.visual_expr(game) {
            Some(expr) => ui.weak(RichText::new(expr).monospace()),
            None => ui.weak("Noch leer: Karten oder eine Anzahl hinzufügen."),
        };
    });
}

fn scope_label(scope: Scope, me: Position) -> String {
    match scope {
        Scope::Player(seat) => position_label(Position::of_seat(seat, me), me),
        Scope::Partner => "mein Mitspieler".into(),
        Scope::AnyOpponent => "irgendein Gegner".into(),
        Scope::AllOpponents => "beide Gegner".into(),
        Scope::AnyOther => "irgendein anderer Spieler".into(),
        Scope::AllOthers => "alle anderen Spieler".into(),
        Scope::AtLeast(k) => format!("mindestens {k} andere Spieler"),
    }
}

/// Partner and opponents only exist in a Sauspiel; elsewhere all others are opponents.
const SAUSPIEL_SCOPES: [Scope; 3] = [Scope::Partner, Scope::AnyOpponent, Scope::AllOpponents];
const GROUP_SCOPES: [Scope; 3] = [Scope::AnyOther, Scope::AllOthers, Scope::AtLeast(2)];

fn scope_picker(ui: &mut Ui, scope: &mut Scope, game: &Game, row: &QueryRow<'_>) {
    let me = row.me;
    let players = Position::ALL.map(|p| Scope::Player(p.seat(me)));
    let sauspiel: &[Scope] = match game.called_sau() {
        Some(_) => &SAUSPIEL_SCOPES,
        None => &[],
    };
    let groups = sauspiel.iter().copied().chain(GROUP_SCOPES);
    // Outside a Sauspiel the opponents are all others; a partner scope is left for the
    // compiler to reject, so the user sees why.
    if game.called_sau().is_none() {
        match *scope {
            Scope::AnyOpponent => *scope = Scope::AnyOther,
            Scope::AllOpponents => *scope = Scope::AllOthers,
            _ => {}
        }
    }
    ComboBox::from_id_salt(("scope", row.index))
        .selected_text(scope_label(*scope, me))
        .width(240.0)
        .show_ui(ui, |ui| {
            for option in groups.into_iter().chain(players) {
                let selected = std::mem::discriminant(scope) == std::mem::discriminant(&option)
                    && (matches!(option, Scope::AtLeast(_)) || *scope == option);
                if ui
                    .selectable_label(selected, scope_label(option, me))
                    .clicked()
                {
                    *scope = option;
                }
            }
        });
    if let Scope::AtLeast(k) = scope {
        ui.add(DragValue::new(k).range(1..=OTHER_PLAYERS).prefix("k = "));
    }
}
