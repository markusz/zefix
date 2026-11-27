// A row of tiles (fixed cards and count rules) with its editor and card picker. Seats and
// visual queries use the same row.

import { createContext, useContext } from "react";
import type { GameInfo } from "../core";
import type { Card, CardSet, CountOp, Tile } from "../model";
import {
  CARD_SETS,
  COUNT_KINDS,
  HAND_SIZE,
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
import { Stepper } from "./Controls";
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
  const addRule = () => {
    onChange([...tiles, { Count: { set: "Trumpf", op: { AtLeast: 1 } } }]);
    // The new rule opens in its editor; the picker makes room for it.
    setEdit({ picker: null, selected: { row, index: tiles.length } });
  };
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
        <button
          className={`add-card ${pickerOpen ? "active" : ""}`}
          onClick={() => setEdit({ selected: null, picker: pickerOpen ? null : row })}
          aria-expanded={pickerOpen}
          title="Karte oder Regel hinzufügen"
        >
          <PlusIcon size={20} />
        </button>
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
          info={info}
          onToggle={toggle}
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

function CardPicker({
  held,
  blocked,
  info,
  onToggle,
  onAddRule,
  onClose,
}: {
  held: Card[];
  blocked: Set<Card>;
  info: GameInfo;
  onToggle: (card: Card) => void;
  onAddRule: () => void;
  onClose: () => void;
}) {
  const full = held.length >= HAND_SIZE;
  let hint = `${held.length} von ${HAND_SIZE} Karten festgelegt`;
  if (full) hint += " · Hand voll";
  else if (blocked.size > 0) hint += " · ausgegraut: liegt bei einem anderen Spieler";
  return (
    <div className="panel picker">
      {info.groups.map((group) => (
        <div key={group.label} className="picker-group">
          <div className="picker-label">{group.label}</div>
          <div className="picker-cards">
            {group.cards.map((card) => {
              const isHeld = held.includes(card);
              const unavailable = !isHeld && (blocked.has(card) || full);
              return (
                <button
                  key={card}
                  className={`tile tile-card small ${isHeld ? "selected" : ""}`}
                  disabled={unavailable}
                  onClick={() => onToggle(card)}
                  title={cardName(card)}
                  aria-pressed={isHeld}
                >
                  <CardFace card={card} />
                </button>
              );
            })}
          </div>
        </div>
      ))}
      <div className="muted small-text">{hint}</div>
      <div className="picker-actions">
        <button
          className="button"
          onClick={onAddRule}
          title="Wie viele einer Sorte, z. B. mind. 3 Gras oder keine Herz"
        >
          + Regel
        </button>
        <button className="button" onClick={onClose}>
          Fertig
        </button>
      </div>
    </div>
  );
}
