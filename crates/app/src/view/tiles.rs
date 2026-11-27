//! A row of tiles (cards and count conditions) with inline editor and card picker.
//! Used for seats and visual queries alike.

use super::range_slider::range_slider;
use crate::model::{self, CardSet, CountKind, CountOp, Tile, cards_of};
use crate::paint::{self, CardLook, MiniCard, Negation, RuleFace, RuleSymbol};
use crate::theme;
use eframe::egui::{self, Button, Color32, ComboBox, Rect, Sense, Slider, Stroke, Ui, pos2, vec2};
use zefix_core::cards::{Card, Cards, Rank};
use zefix_core::game::{Game, Seat};
use zefix_core::scenario::HAND_SIZE;

/// The placeholder is wider than a card so its two buttons fit.
const PLACEHOLDER_CARDS: f32 = 1.5;
const MIN_PLACEHOLDER_WIDTH: f32 = 84.0;
const WRAP_SLACK: f32 = 8.0;
const MINI_CARD_GAP: f32 = 5.0;
const PLACEHOLDER_PADDING: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Seat(Seat),
    Query(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub target: Target,
    pub index: usize,
}

#[derive(Debug, Default)]
pub struct EditState {
    pub selected: Option<Selection>,
    pub picker: Option<Target>,
}

impl EditState {
    /// Forgets selections that point at `target`, e.g. after it was removed or reordered.
    pub fn release(&mut self, target: Target) {
        if self.selected.is_some_and(|s| s.target == target) {
            self.selected = None;
        }
        if self.picker == Some(target) {
            self.picker = None;
        }
    }
}

pub struct TileRow<'a> {
    pub target: Target,
    pub tiles: &'a mut Vec<Tile>,
    /// Cards that may not be picked here because another seat holds them.
    pub blocked: Cards,
    /// Colour of the area the row belongs to, for its action buttons.
    pub accent: Color32,
}

/// Scale at which a full hand plus the placeholder fills one line.
fn hand_scale(ui: &Ui) -> f32 {
    // One gap per item plus slack, so rounding never pushes the placeholder to a new line.
    let gaps = ui.spacing().item_spacing.x * (HAND_SIZE as f32 + 1.0) + WRAP_SLACK;
    let cards = HAND_SIZE as f32 + PLACEHOLDER_CARDS;
    ((ui.available_width() - gaps) / (paint::CARD_SIZE.x * cards))
        .clamp(paint::MIN_HAND_SCALE, paint::MAX_HAND_SCALE)
}

pub fn show(ui: &mut Ui, game: &Game, edit: &mut EditState, mut row: TileRow<'_>) {
    let target = row.target;
    let scale = hand_scale(ui);
    let fixed = cards_of(row.tiles);
    ui.horizontal_wrapped(|ui| {
        for (index, tile) in row.tiles.iter().enumerate() {
            let selection = Selection { target, index };
            let selected = edit.selected == Some(selection);
            let style = TileStyle {
                selected,
                fixed,
                accent: row.accent,
                scale,
            };
            if tile_widget(ui, game, *tile, style).clicked() {
                edit.selected = (!selected).then_some(selection);
            }
        }
        let style = TileStyle {
            selected: false,
            fixed,
            accent: row.accent,
            scale,
        };
        placeholder(ui, edit, target, row.tiles, style);
    });
    editor(ui, game, edit, &mut row);
    if edit.picker == Some(target)
        && let Some(card) = picker(ui, game, row.tiles, row.blocked)
    {
        toggle_card(game, row.tiles, card);
        // Toggling reorders the tiles, so a selected index would point elsewhere.
        if edit.selected.is_some_and(|s| s.target == target) {
            edit.selected = None;
        }
    }
}

#[derive(Clone, Copy)]
struct TileStyle {
    selected: bool,
    /// Fixed cards of the row; rules count on top of them.
    fixed: Cards,
    accent: Color32,
    scale: f32,
}

fn tile_widget(ui: &mut Ui, game: &Game, tile: Tile, style: TileStyle) -> egui::Response {
    let look = if style.selected {
        CardLook::Selected
    } else {
        CardLook::Normal
    };
    match tile {
        Tile::Card(card) => paint::card(ui, card, look, style.scale)
            .on_hover_text(format!("Karte {card} · klicken zum Bearbeiten")),
        Tile::Count { set, op } => {
            let face = rule_face(game, set, op);
            paint::rule_card(ui, &face, look, style.scale, style.accent).on_hover_text(rule_hint(
                game,
                set,
                op,
                style.fixed,
            ))
        }
    }
}

/// "Trumpf: 2–5 weitere (neben EO, GO)" when the row already holds cards of the set.
fn rule_hint(game: &Game, set: CardSet, op: CountOp, fixed: Cards) -> String {
    let in_set = model::set_cards(set, game) & fixed;
    let besides = if in_set.is_empty() {
        String::new()
    } else {
        let names: Vec<String> = game.sorted(in_set).iter().map(|c| c.to_string()).collect();
        format!(" weitere (neben {})", names.join(", "))
    };
    let name = match set {
        CardSet::Trumpf => format!("Trumpf ({})", trump_description(game)),
        _ => set.label().to_string(),
    };
    format!("{name}: {}{besides} · klicken zum Bearbeiten", op.label())
}

/// E.g. "Ober, Unter, Herz" in a Sauspiel or "Unter" in a Wenz.
fn trump_description(game: &Game) -> String {
    let ranks = [(Rank::Ober, "Ober"), (Rank::Unter, "Unter")]
        .into_iter()
        .filter(|&(rank, _)| (Cards::of_rank(rank) - game.trumps).is_empty())
        .map(|(_, name)| name);
    let suit = game.trump_suit().map(|s| s.display_name());
    ranks.chain(suit).collect::<Vec<_>>().join(", ")
}

fn rule_face(game: &Game, set: CardSet, op: CountOp) -> RuleFace<'_> {
    let rank = |r: Rank| RuleSymbol::Rank(r.symbol());
    let symbol = match set {
        CardSet::Suit(suit) => RuleSymbol::Suit(suit),
        CardSet::Trumpf => RuleSymbol::Trumpf,
        CardSet::Farbe => RuleSymbol::Suits(&game.plain_suits),
        CardSet::Ober => rank(Rank::Ober),
        CardSet::Unter => rank(Rank::Unter),
        CardSet::Sau => rank(Rank::Sau),
        CardSet::Zehn => rank(Rank::Zehn),
        CardSet::Koenig => rank(Rank::Koenig),
    };
    let negation = match op {
        CountOp::Zero | CountOp::Exactly(0) => Negation::Struck,
        _ => Negation::Plain,
    };
    RuleFace {
        symbol,
        badge: op.badge(),
        negation,
    }
}

/// Card-shaped outline with the two ways to add something to the hand.
fn placeholder(
    ui: &mut Ui,
    edit: &mut EditState,
    target: Target,
    tiles: &mut Vec<Tile>,
    style: TileStyle,
) {
    let accent = style.accent;
    let card = paint::CARD_SIZE * style.scale;
    let size = vec2(
        (card.x * PLACEHOLDER_CARDS).max(MIN_PLACEHOLDER_WIDTH),
        card.y,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    paint::dashed_outline(ui.painter(), rect, accent);
    let inner = rect.shrink(PLACEHOLDER_PADDING);
    let half = (inner.height() - PLACEHOLDER_PADDING) / 2.0;
    let top = Rect::from_min_size(inner.min, vec2(inner.width(), half));
    let bottom = Rect::from_min_size(
        pos2(inner.left(), inner.bottom() - half),
        vec2(inner.width(), half),
    );
    let open = edit.picker == Some(target);
    let label = if open { "Fertig" } else { "+ Karte" };
    let cards = placeholder_button(ui, top, label, MiniCard::Solid, accent)
        .on_hover_text("Eine bestimmte Karte, z. B. der Gras-Ober");
    if cards.clicked() {
        edit.picker = (!open).then_some(target);
    }
    let rule = placeholder_button(ui, bottom, "+ Regel", MiniCard::Dotted, accent)
        .on_hover_text("Wie viele einer Sorte, z. B. mind. 3 Gras oder keine Herz");
    if rule.clicked() {
        tiles.push(Tile::Count {
            set: CardSet::Trumpf,
            op: CountOp::AT_LEAST_ONE,
        });
        edit.selected = Some(Selection {
            target,
            index: tiles.len() - 1,
        });
    }
}

/// Button whose label starts with a tiny card outline: solid for cards, dotted for rules,
/// matching what the click adds to the hand.
fn placeholder_button(
    ui: &mut Ui,
    rect: Rect,
    label: &str,
    icon: MiniCard,
    accent: Color32,
) -> egui::Response {
    let frame = Button::new("")
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, accent.gamma_multiply(0.6)));
    let response = ui.put(rect, frame);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(label.into(), font, accent);
    let icon_size = vec2(galley.size().y * 0.6, galley.size().y * 0.85);
    let total = icon_size.x + MINI_CARD_GAP + galley.size().x;
    let left = response.rect.center().x - total / 2.0;
    let icon_rect = Rect::from_center_size(
        pos2(left + icon_size.x / 2.0, response.rect.center().y),
        icon_size,
    );
    paint::mini_card(ui.painter(), icon_rect, icon, accent);
    let text_pos = pos2(
        left + icon_size.x + MINI_CARD_GAP,
        response.rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, accent);
    response
}

enum EditorAction {
    Keep,
    Close,
    Remove,
}

#[derive(Clone, Copy)]
struct EditorCtx {
    sel: Selection,
    fixed: Cards,
    accent: Color32,
}

fn editor(ui: &mut Ui, game: &Game, edit: &mut EditState, row: &mut TileRow<'_>) {
    let target = row.target;
    let tiles = &mut *row.tiles;
    let Some(sel) = edit.selected.filter(|s| s.target == target) else {
        return;
    };
    let ctx = EditorCtx {
        sel,
        fixed: cards_of(tiles),
        accent: row.accent,
    };
    let Some(tile) = tiles.get_mut(sel.index) else {
        edit.selected = None;
        return;
    };
    let action = theme::inset(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            match tile {
                Tile::Count { set, op } => count_editor(ui, game, set, op, ctx),
                Tile::Card(card) => {
                    ui.label(format!("Karte {card}"));
                }
            }
            ui.separator();
            if theme::danger(ui, "Weglegen").clicked() {
                EditorAction::Remove
            } else if ui.button("Fertig").clicked() {
                EditorAction::Close
            } else {
                EditorAction::Keep
            }
        })
        .inner
    });
    match action {
        EditorAction::Keep => {}
        EditorAction::Close => edit.selected = None,
        EditorAction::Remove => {
            tiles.remove(sel.index);
            edit.selected = None;
        }
    }
}

/// Reads as a sentence: "[Ober ▾] [zwischen ▾] ●━━━● 1 bis 3".
fn count_editor(ui: &mut Ui, game: &Game, set: &mut CardSet, op: &mut CountOp, ctx: EditorCtx) {
    let before = *set;
    ComboBox::from_id_salt(("count_set", ctx.sel.target, ctx.sel.index))
        .selected_text(set.label())
        .show_ui(ui, |ui| {
            for option in CardSet::ALL {
                ui.selectable_value(set, option, option.label());
            }
        });
    let cap = model::rule_cap(*set, ctx.fixed, game);
    if *set != before {
        *op = op.clamped(cap);
    }
    let kind = op.kind();
    ComboBox::from_id_salt(("count_kind", ctx.sel.target, ctx.sel.index))
        .selected_text(kind.label())
        .show_ui(ui, |ui| {
            for option in CountKind::ALL {
                if ui
                    .selectable_label(option == kind, option.label())
                    .clicked()
                {
                    *op = op.with_kind(option, cap);
                }
            }
        });
    match op {
        CountOp::AtLeast(n) => {
            ui.add(Slider::new(n, 1..=cap));
        }
        CountOp::AtMost(n) | CountOp::Exactly(n) => {
            ui.add(Slider::new(n, 0..=cap));
        }
        CountOp::Between { min, max } => {
            range_slider(ui, min, max, cap, ctx.accent);
            ui.label(format!("{min} bis {max}"));
        }
        CountOp::Zero => {}
    }
}

/// Returns the card the user clicked, if any.
fn picker(ui: &mut Ui, game: &Game, tiles: &[Tile], blocked: Cards) -> Option<Card> {
    let held = cards_of(tiles);
    let mut toggled: Option<Card> = None;
    theme::inset(ui, |ui| {
        let groups = std::iter::once(("Trumpf", game.trumps)).chain(
            game.plain_suits
                .iter()
                .map(|&s| (s.display_name(), game.suit_set(s))),
        );
        for (label, cards) in groups {
            ui.weak(label);
            ui.horizontal_wrapped(|ui| {
                for card in game.sorted(cards) {
                    let look = card_look(card, held, blocked);
                    if paint::card(ui, card, look, 1.0).clicked() {
                        toggled = Some(card);
                    }
                }
            });
        }
        ui.weak(picker_hint(held, blocked));
    });
    toggled
}

fn picker_hint(held: Cards, blocked: Cards) -> String {
    let mut hint = format!("{} von {HAND_SIZE} Karten festgelegt", held.len());
    if held.len() >= HAND_SIZE {
        hint += " · Hand voll";
    } else if !blocked.is_empty() {
        hint += " · grau: liegt bei einem anderen Spieler";
    }
    hint
}

fn card_look(card: Card, held: Cards, blocked: Cards) -> CardLook {
    if held.contains(card) {
        CardLook::Selected
    } else if blocked.contains(card) || held.len() >= HAND_SIZE {
        CardLook::Unavailable
    } else {
        CardLook::Normal
    }
}

/// Adds or removes `card`, keeping card tiles first and in game order.
fn toggle_card(game: &Game, tiles: &mut Vec<Tile>, card: Card) {
    let mut cards = cards_of(tiles);
    if cards.contains(card) {
        cards = cards - card.mask();
    } else {
        cards |= card.mask();
    }
    let counts: Vec<Tile> = tiles
        .iter()
        .filter(|t| matches!(t, Tile::Count { .. }))
        .copied()
        .collect();
    *tiles = game.sorted(cards).into_iter().map(Tile::Card).collect();
    tiles.extend(counts);
}
