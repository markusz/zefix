// The scenario as the UI edits it. Mirrors the serde JSON of `zefix_model::model::Model`
// exactly: share links are that JSON, so a change here must match the Rust side (and bump the
// link key there if old links would break). Meaning — what tiles compile to — lives in Rust;
// this file only holds what drawing and editing need.

export type Suit = "eichel" | "gras" | "herz" | "schellen";
export type Seat = "me" | "left" | "across" | "right";
export type GameChoice = "Sauspiel" | "Solo" | "Wenz" | "Farbwenz" | "Geier" | "Farbgeier";
export type Position = "First" | "Second" | "Third" | "Fourth";
/** Short card name as in the query language: `EO`, `S10`, `HA`. */
export type Card = string;

export type CardSet =
  | "Trumpf"
  | "Farbe"
  | "Ober"
  | "Unter"
  | "Sau"
  | "Zehn"
  | "Koenig"
  | { Suit: Suit };

export type CountOp =
  | { AtLeast: number }
  | { AtMost: number }
  | { Exactly: number }
  | { Between: { min: number; max: number } }
  | "Zero";

export type Tile = { Card: Card } | { Count: { set: CardSet; op: CountOp } };

export type Scope =
  | { Player: Seat }
  | "Partner"
  | "AnyOther"
  | "AllOthers"
  | { AtLeast: number }
  | "AnyOpponent"
  | "AllOpponents";

export type QueryMode = "Visual" | "Text";

export interface QueryModel {
  name: string;
  mode: QueryMode;
  scope: Scope;
  tiles: Tile[];
  text: string;
}

export interface GameSetup {
  choice: GameChoice;
  suit: Suit;
}

export interface Model {
  game: GameSetup;
  my_position: Position;
  /** Indexed like `SEATS`, so my hand stays mine when I change position. */
  seats: { tiles: Tile[] }[];
  queries: QueryModel[];
  filter_text: string;
}

export const SUITS: Suit[] = ["eichel", "gras", "herz", "schellen"];
export const SEATS: Seat[] = ["me", "left", "across", "right"];
export const POSITIONS: Position[] = ["First", "Second", "Third", "Fourth"];
export const HAND_SIZE = 8;
export const OTHER_PLAYERS = 3;

export const GAMES: { choice: GameChoice; label: string }[] = [
  { choice: "Sauspiel", label: "Sauspiel" },
  { choice: "Solo", label: "Solo" },
  { choice: "Wenz", label: "Wenz" },
  { choice: "Farbwenz", label: "Farbwenz" },
  { choice: "Geier", label: "Geier" },
  { choice: "Farbgeier", label: "Farbgeier" },
];

export const SUIT_NAMES: Record<Suit, string> = {
  eichel: "Eichel",
  gras: "Gras",
  herz: "Herz",
  schellen: "Schellen",
};

const CALLABLE: Suit[] = ["eichel", "gras", "schellen"];

/** Suits the player may pick; empty if the game has none. */
export function gameSuits(choice: GameChoice): Suit[] {
  switch (choice) {
    case "Sauspiel":
      return CALLABLE;
    case "Solo":
    case "Farbwenz":
    case "Farbgeier":
      return SUITS;
    case "Wenz":
    case "Geier":
      return [];
  }
}

export function suitLabel(choice: GameChoice): string {
  return choice === "Sauspiel" ? "Gerufene Sau" : "Trumpf";
}

/** Keeps the suit valid after the game changed (e.g. no Herz-Sau to call). */
export function normalizeGame(setup: GameSetup): GameSetup {
  const suits = gameSuits(setup.choice);
  const first = suits[0];
  return first !== undefined && !suits.includes(setup.suit) ? { ...setup, suit: first } : setup;
}

const SEAT_NAMES: Record<Seat, string> = {
  me: "Ich",
  left: "Links",
  across: "Gegenüber",
  right: "Rechts",
};

export function seatName(seat: Seat): string {
  return SEAT_NAMES[seat];
}

/** Play runs clockwise, so the player after me sits on my left. */
export function seatAt(position: Position, me: Position): Seat {
  const i = (POSITIONS.indexOf(position) - POSITIONS.indexOf(me) + 4) % 4;
  return SEATS[i] ?? "me";
}

export function positionOf(seat: Seat, me: Position): Position {
  return POSITIONS[(SEATS.indexOf(seat) + POSITIONS.indexOf(me)) % 4] ?? "First";
}

// Card sets

export const CARD_SETS: CardSet[] = [
  "Trumpf",
  "Farbe",
  "Ober",
  "Unter",
  "Sau",
  "Zehn",
  "Koenig",
  { Suit: "eichel" },
  { Suit: "gras" },
  { Suit: "herz" },
  { Suit: "schellen" },
];

const SET_LABELS: Record<Exclude<CardSet, { Suit: Suit }>, string> = {
  Trumpf: "Trumpf",
  Farbe: "Farbe",
  Ober: "Ober",
  Unter: "Unter",
  Sau: "Sau",
  Zehn: "Zehner",
  Koenig: "König",
};

export function setLabel(set: CardSet): string {
  return typeof set === "string" ? SET_LABELS[set] : SUIT_NAMES[set.Suit];
}

/** Name in the query language, the key of `GameInfo.sets`. */
export function setIdent(set: CardSet): string {
  return typeof set === "string" ? set.toLowerCase() : set.Suit;
}

export function sameSet(a: CardSet, b: CardSet): boolean {
  return setIdent(a) === setIdent(b);
}

// Count rules

export type CountKind = "AtLeast" | "AtMost" | "Exactly" | "Between" | "Zero";

export const COUNT_KINDS: { kind: CountKind; label: string }[] = [
  { kind: "AtLeast", label: "mindestens" },
  { kind: "AtMost", label: "höchstens" },
  { kind: "Exactly", label: "genau" },
  { kind: "Between", label: "zwischen" },
  { kind: "Zero", label: "keine" },
];

export function countKind(op: CountOp): CountKind {
  return op === "Zero" ? "Zero" : (Object.keys(op)[0] as CountKind);
}

/** The number the user last chose, carried over when the kind changes. */
function countValue(op: CountOp): number {
  if (op === "Zero") return 0;
  if ("AtLeast" in op) return op.AtLeast;
  if ("AtMost" in op) return op.AtMost;
  if ("Exactly" in op) return op.Exactly;
  return op.Between.min;
}

export function withKind(op: CountOp, kind: CountKind, cap: number): CountOp {
  const v = Math.min(countValue(op), cap);
  switch (kind) {
    case "AtLeast":
      return { AtLeast: Math.max(v, 1) };
    case "AtMost":
      return { AtMost: v };
    case "Exactly":
      return { Exactly: v };
    case "Between":
      return { Between: { min: v, max: cap } };
    case "Zero":
      return "Zero";
  }
}

/** Keeps values within `0..=cap`, e.g. after switching the set from Trumpf to Ober. */
export function clampOp(op: CountOp, cap: number): CountOp {
  if (op === "Zero") return op;
  if ("AtLeast" in op) return { AtLeast: Math.min(op.AtLeast, cap) };
  if ("AtMost" in op) return { AtMost: Math.min(op.AtMost, cap) };
  if ("Exactly" in op) return { Exactly: Math.min(op.Exactly, cap) };
  return { Between: { min: Math.min(op.Between.min, cap), max: Math.min(op.Between.max, cap) } };
}

export function opLabel(op: CountOp): string {
  if (op === "Zero") return "keine";
  if ("AtLeast" in op) return `mind. ${op.AtLeast}`;
  if ("AtMost" in op) return `höchstens ${op.AtMost}`;
  if ("Exactly" in op) return `genau ${op.Exactly}`;
  return `${op.Between.min}–${op.Between.max}`;
}

/** Short form for the badge on a rule card. */
export function opBadge(op: CountOp): string {
  if (op === "Zero") return "0";
  if ("AtLeast" in op) return `${op.AtLeast}+`;
  if ("AtMost" in op) return `≤${op.AtMost}`;
  if ("Exactly" in op) return `${op.Exactly}`;
  return `${op.Between.min}–${op.Between.max}`;
}

export function isNegation(op: CountOp): boolean {
  return op === "Zero" || ("Exactly" in op && op.Exactly === 0);
}

// Tiles

export function cardsOf(tiles: Tile[]): Card[] {
  return tiles.flatMap((t) => ("Card" in t ? [t.Card] : []));
}

/** Cards pinned to any seat other than `seat`. */
export function cardsElsewhere(model: Model, seat: Seat): Set<Card> {
  const others = SEATS.flatMap((s, i) => (s === seat ? [] : cardsOf(model.seats[i]?.tiles ?? [])));
  return new Set(others);
}

/** Adds or removes `card`, keeping card tiles first and in game order. */
export function toggleCard(tiles: Tile[], card: Card, order: Card[]): Tile[] {
  const held = new Set(cardsOf(tiles));
  if (held.has(card)) held.delete(card);
  else held.add(card);
  const cards = order.filter((c) => held.has(c)).map((c): Tile => ({ Card: c }));
  return [...cards, ...tiles.filter((t) => "Count" in t)];
}

// Scopes

/** Partner and opponents only exist in a Sauspiel; elsewhere all others are opponents. */
export function scopeOptions(sauspiel: boolean, me: Position): Scope[] {
  const groups: Scope[] = [
    ...(sauspiel ? (["Partner", "AnyOpponent", "AllOpponents"] as Scope[]) : []),
    "AnyOther",
    "AllOthers",
    { AtLeast: 2 },
  ];
  const players = POSITIONS.map((p): Scope => ({ Player: seatAt(p, me) }));
  return [...groups, ...players];
}

export function scopeKey(scope: Scope): string {
  if (typeof scope === "string") return scope;
  return "Player" in scope ? `Player:${scope.Player}` : "AtLeast";
}

export function scopeLabel(scope: Scope): string {
  if (typeof scope === "string") {
    switch (scope) {
      case "Partner":
        return "mein Mitspieler";
      case "AnyOpponent":
        return "irgendein Gegner";
      case "AllOpponents":
        return "beide Gegner";
      case "AnyOther":
        return "irgendein anderer Spieler";
      case "AllOthers":
        return "alle anderen Spieler";
    }
  }
  return "Player" in scope
    ? scope.Player === "me"
      ? "ich"
      : seatName(scope.Player)
    : `mindestens ${scope.AtLeast} andere Spieler`;
}

/** Outside a Sauspiel there are no two opponents; their scopes become "the others". */
export function fitScope(scope: Scope, sauspiel: boolean): Scope {
  if (sauspiel) return scope;
  if (scope === "AnyOpponent") return "AnyOther";
  if (scope === "AllOpponents") return "AllOthers";
  return scope;
}

export function visualQuery(name: string, scope: Scope, tiles: Tile[]): QueryModel {
  return { name, mode: "Visual", scope, tiles, text: "" };
}
