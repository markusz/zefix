// Typed access to the Rust side (`zefix_model::api`, compiled to WebAssembly). Every call
// passes JSON; fallible calls come back as `{ok}` or `{err}`.

import init, * as wasm from "./wasm/zefix_wasm.js";
import type { Card, GameSetup, Model, Seat, Suit } from "./model";

export type Outcome<T, E = string> = { ok: T } | { err: E };

export interface GameInfo {
  order: Card[];
  groups: { label: string; cards: Card[] }[];
  plain_suits: Suit[];
  sets: { ident: string; cards: Card[] }[];
  called_sau: Card | null;
  trump_description: string;
}

export type Problem =
  | { kind: "filter"; rendered: string }
  | { kind: "query"; name: string; rendered: string }
  | { kind: "noQueries" }
  | { kind: "invalidGame"; message: string }
  | { kind: "other"; message: string };

export interface Compiled {
  filter: string | null;
  fixed: Card[];
  queries: { name: string; expr: string | null }[];
  problem: Problem | null;
}

export interface Report {
  finished: boolean;
  fraction: number;
  accepted: number;
  attempts: number;
  results: QueryResult[];
}

export interface QueryResult {
  name: string;
  p: number;
  /** Half width of the 95 % interval. */
  margin: number;
  hits: number;
  examples: { hands: Card[][]; partner: Seat | null }[];
}

export interface Imported {
  model: Model;
  dropped: string[];
}

let ready: Promise<void> | undefined;

/** Loads the WebAssembly module once; every other call needs it. */
export function load(): Promise<void> {
  ready ??= init().then(() => undefined);
  return ready;
}

function parse<T>(json: string): T {
  return JSON.parse(json) as T;
}

export const core = {
  defaultModel: (): Model => parse(wasm.defaultModel()),
  gameInfo: (setup: GameSetup): Outcome<GameInfo> => parse(wasm.gameInfo(JSON.stringify(setup))),
  compile: (model: Model): Outcome<Compiled> => parse(wasm.compile(JSON.stringify(model))),
  linkFragment: (model: Model): Outcome<string> => parse(wasm.linkFragment(JSON.stringify(model))),
  fromLink: (text: string): Outcome<Model> | null => parse(wasm.fromLink(text)),
  toToml: (model: Model): Outcome<string> => parse(wasm.toToml(JSON.stringify(model))),
  importText: (text: string): Outcome<Imported> => parse(wasm.importText(text)),
};

export { wasm };
