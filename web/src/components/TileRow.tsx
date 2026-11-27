// A row of tiles (fixed cards and count rules) with its editor and card picker. Seats and
// visual queries use the same row.

import { createContext, useContext, useState } from "react";
import type { GameInfo } from "../core";
import type { Card, CardSet, CountOp, Suit, Tile } from "../model";
import {
  CARD_SETS,
  COUNT_KINDS,
  HAND_SIZE,
  SUITS,
  cardsOf,
  clampOp,
  countKind,
  opLabel,
  sameSet,
  setIdent,
  setLabel,
  toggleCard,
  withKind,
} from "../model";
import { CardFace, RuleFace, cardName } from "./Cards";
import { Segmented, Stepper } from "./Controls";
import { PlusIcon } from "./Icons";

/** Which tile is being edited and which row has its card picker open; one of each at most. */
export interface EditState {
  selected: { row: string; index: number } | null;
  picker: string | null;
}

export const EditContext = createContext<{
  edit: EditState;
  setEdit: (edit: EditState) => void;
}>({ edit: { selected: null, picker: null }, setEdit: () => {} });

export type Accent = "me" | "seats" | "queries";

interface Props {
  /** Identifies the row in `EditState`, e.g. `seat:left` or `query:2`. */
  row: string;
  tiles: Tile[];
  onChange: (tiles: Tile[]) => void;
  info: GameInfo;
  /** Cards another seat holds; they cannot be picked here. */
  blocked?: Set<Card>;
  accent: Accent;
}

export function TileRow({ row, tiles, onChange, info, blocked = new Set(), accent }: Props) {
  const { edit, setEdit } = useContext(EditContext);
  const fixed = cardsOf(tiles);
  const selected = edit.selected?.row === row ? edit.selected.index : null;
  const pickerOpen = edit.picker === row;

  const select = (index: number) =>
    setEdit({ ...edit, selected: selected === index ? null : { row, index } });
  const addRule = (set: CardSet, op: CountOp) => onChange([...tiles, { Count: { set, op } }]);
  const toggle = (card: Card) => {
    onChange(toggleCard(tiles, card, info.order));
    // Toggling reorders the tiles, so a selected index would point elsewhere.
    setEdit({ ...edit, selected: selected === null ? edit.selected : null });
  };

  const tile = selected !== null ? tiles[selected] : undefined;
  return (
    <div className={`tile-row accent-${accent}`}>
      <div className="tiles">
        {tiles.map((t, index) => (
          <button
            key={index}
            className={`tile ${"Card" in t ? "tile-card" : "tile-rule"} ${selected === index ? "selected" : ""}`}
            onClick={() => select(index)}
            title={"Card" in t ? cardName(t.Card) : ruleHint(t.Count.set, t.Count.op, fixed, info)}
          >
            {"Card" in t ? (
              <CardFace card={t.Card} />
            ) : (
              <RuleFace set={t.Count.set} op={t.Count.op} plainSuits={info.plain_suits} />
            )}
          </button>
        ))}
        {/* A full hand leaves nothing to add; its cards are removed from their own editor. */}
        {fixed.length < HAND_SIZE && (
          <button
            className={`add-card ${pickerOpen ? "active" : ""}`}
            onClick={() => setEdit({ selected: null, picker: pickerOpen ? null : row })}
            aria-expanded={pickerOpen}
            title="Karte oder Regel hinzufügen"
          >
            <PlusIcon size={20} />
          </button>
        )}
      </div>
      {tile && selected !== null && (
        <TileEditor
          tile={tile}
          fixed={fixed}
          info={info}
          onChange={(next) => onChange(tiles.map((t, i) => (i === selected ? next : t)))}
          onRemove={() => {
            onChange(tiles.filter((_, i) => i !== selected));
            setEdit({ ...edit, selected: null });
          }}
          onClose={() => setEdit({ ...edit, selected: null })}
        />
      )}
      {pickerOpen && (
        <CardPicker
          held={fixed}
          blocked={blocked}
          onToggle={toggle}
          cap={(set) => ruleCap(set, fixed, info)}
          onAddRule={addRule}
          onClose={() => setEdit({ ...edit, picker: null })}
        />
      )}
    </div>
  );
}

function setCards(set: CardSet, info: GameInfo): Card[] {
  return info.sets.find((s) => s.ident === setIdent(set))?.cards ?? [];
}

/** Most additional cards of `set` a rule can ask for next to the `fixed` cards. */
function ruleCap(set: CardSet, fixed: Card[], info: GameInfo): number {
  const unfixed = setCards(set, info).filter((c) => !fixed.includes(c)).length;
  return Math.min(unfixed, Math.max(0, HAND_SIZE - fixed.length));
}

/** "Trumpf (Ober, Unter, Herz): mind. 2 weitere (neben EO, GO)". */
function ruleHint(set: CardSet, op: CountOp, fixed: Card[], info: GameInfo): string {
  const inSet = setCards(set, info).filter((c) => fixed.includes(c));
  const besides = inSet.length > 0 ? ` weitere (neben ${inSet.join(", ")})` : "";
  const name = set === "Trumpf" ? `Trumpf (${info.trump_description})` : setLabel(set);
  return `${name}: ${opLabel(op)}${besides}`;
}

interface EditorProps {
  tile: Tile;
  fixed: Card[];
  info: GameInfo;
  onChange: (tile: Tile) => void;
  onRemove: () => void;
  onClose: () => void;
}

/** Reads as a sentence: "[Ober ▾] [zwischen ▾] − 1 + bis − 3 +". */
function TileEditor({ tile, fixed, info, onChange, onRemove, onClose }: EditorProps) {
  return (
    <div className="panel editor">
      {"Card" in tile ? (
        <span className="editor-card">{cardName(tile.Card)}</span>
      ) : (
        <RuleEditor
          set={tile.Count.set}
          op={tile.Count.op}
          cap={(set) => ruleCap(set, fixed, info)}
          onChange={(set, op) => onChange({ Count: { set, op } })}
        />
      )}
      <span className="editor-actions">
        <button className="button danger" onClick={onRemove}>
          Entfernen
        </button>
        <button className="button" onClick={onClose}>
          Fertig
        </button>
      </span>
    </div>
  );
}

function RuleEditor({
  set,
  op,
  cap,
  onChange,
}: {
  set: CardSet;
  op: CountOp;
  cap: (set: CardSet) => number;
  onChange: (set: CardSet, op: CountOp) => void;
}) {
  const max = cap(set);
  const kind = countKind(op);
  return (
    <span className="rule-editor">
      <select
        className="select"
        aria-label="Sorte"
        value={setIdent(set)}
        onChange={(e) => {
          const next = CARD_SETS.find((s) => setIdent(s) === e.target.value) ?? set;
          onChange(next, sameSet(next, set) ? op : clampOp(op, cap(next)));
        }}
      >
        {CARD_SETS.map((s) => (
          <option key={setIdent(s)} value={setIdent(s)}>
            {setLabel(s)}
          </option>
        ))}
      </select>
      <select
        className="select"
        aria-label="Vergleich"
        value={kind}
        onChange={(e) => {
          const next = COUNT_KINDS.find((k) => k.kind === e.target.value)?.kind ?? kind;
          onChange(set, withKind(op, next, max));
        }}
      >
        {COUNT_KINDS.map((k) => (
          <option key={k.kind} value={k.kind}>
            {k.label}
          </option>
        ))}
      </select>
      {op !== "Zero" && "AtLeast" in op && (
        <Stepper value={op.AtLeast} min={1} max={max} onChange={(n) => onChange(set, { AtLeast: n })} />
      )}
      {op !== "Zero" && "AtMost" in op && (
        <Stepper value={op.AtMost} min={0} max={max} onChange={(n) => onChange(set, { AtMost: n })} />
      )}
      {op !== "Zero" && "Exactly" in op && (
        <Stepper value={op.Exactly} min={0} max={max} onChange={(n) => onChange(set, { Exactly: n })} />
      )}
      {op !== "Zero" && "Between" in op && (
        <>
          <Stepper
            value={op.Between.min}
            min={0}
            max={op.Between.max}
            onChange={(n) => onChange(set, { Between: { ...op.Between, min: n } })}
          />
          <span className="muted">bis</span>
          <Stepper
            value={op.Between.max}
            min={op.Between.min}
            max={max}
            onChange={(n) => onChange(set, { Between: { ...op.Between, max: n } })}
          />
        </>
      )}
    </span>
  );
}

const RANKS = ["O", "U", "A", "10", "K", "9", "8", "7"];
const SUIT_LETTER: Record<Suit, string> = { eichel: "E", gras: "G", herz: "H", schellen: "S" };

type AddMode = "card" | "rule";

/** One way in, switched between picking cards (one row per suit) and building a rule. */
function CardPicker({
  held,
  blocked,
  cap,
  onToggle,
  onAddRule,
  onClose,
}: {
  held: Card[];
  blocked: Set<Card>;
  cap: (set: CardSet) => number;
  onToggle: (card: Card) => void;
  onAddRule: (set: CardSet, op: CountOp) => void;
  onClose: () => void;
}) {
  const [mode, setMode] = useState<AddMode>("card");
  const full = held.length >= HAND_SIZE;
  return (
    <div className="panel picker">
      <div className="picker-head">
        <Segmented
          label="Hinzufügen"
          value={mode}
          onChange={setMode}
          options={[
            { value: "card", label: "Karte", title: "Eine bestimmte Karte, z. B. der Gras-Ober" },
            { value: "rule", label: "Regel", title: "Wie viele einer Sorte, z. B. mind. 3 Trumpf oder keine Herz" },
          ]}
        />
      </div>
      {mode === "card" ? (
      <div className="card-matrix">
        {SUITS.map((suit) => (
          <div key={suit} className="matrix-row">
            {RANKS.map((rank) => {
              const card = `${SUIT_LETTER[suit]}${rank}`;
              const isHeld = held.includes(card);
              // Grey means taken: by this hand (tap to put it back) or by another seat.
              const taken = isHeld || blocked.has(card);
              return (
                <button
                  key={card}
                  className={`tile tile-card matrix-card ${taken ? "taken" : ""}`}
                  disabled={!isHeld && (blocked.has(card) || full)}
                  onClick={() => onToggle(card)}
                  title={isHeld ? `${cardName(card)} · zurücklegen` : blocked.has(card) ? `${cardName(card)} · liegt bei einem anderen Spieler` : cardName(card)}
                  aria-pressed={isHeld}
                >
                  <CardFace card={card} />
                </button>
              );
            })}
          </div>
        ))}
      </div>
      ) : (
        <DraftRule cap={cap} onAdd={onAddRule} />
      )}
      <div className="picker-actions">
        <button className="button" onClick={onClose}>
          Fertig
        </button>
      </div>
    </div>
  );
}

/** "[Trumpf ▾] [mindestens ▾] [− 2 +] Hinzufügen", added as a new rule tile. */
function DraftRule({ cap, onAdd }: { cap: (set: CardSet) => number; onAdd: (set: CardSet, op: CountOp) => void }) {
  const [draft, setDraft] = useState<{ set: CardSet; op: CountOp }>({ set: "Trumpf", op: { AtLeast: 2 } });
  return (
    <div className="draft">
      <RuleEditor
        set={draft.set}
        op={draft.op}
        cap={cap}
        onChange={(set, op) => setDraft({ set, op })}
      />
      <button className="button accent" onClick={() => onAdd(draft.set, draft.op)}>
        <PlusIcon size={16} /> Hinzufügen
      </button>
    </div>
  );
}
