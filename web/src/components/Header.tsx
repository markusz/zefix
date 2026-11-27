// The game, my position and the run length: everything that frames the scenario.

import type { GameChoice, GameSetup, Model, Position } from "../model";
import { GAMES, POSITIONS, SUIT_NAMES, gameSuits, normalizeGame, suitLabel } from "../model";
import { SuitIcon } from "./Cards";
import { Segmented } from "./Controls";
import { Hint } from "./Hint";
import { HelpIcon, ShareIcon } from "./Icons";

/** A run length named by what it buys: how precise the numbers get. */
export const PRECISIONS = [
  { name: "Sehr schnell", iterations: 10_000 },
  { name: "Schnell", iterations: 100_000 },
  { name: "Normal", iterations: 1_000_000 },
  { name: "Genau", iterations: 10_000_000 },
];

export function Brand({ onHelp }: { onHelp: () => void }) {
  return (
    <>
      <span className="brand">
        <img src={`${import.meta.env.BASE_URL}logo/zefix.svg`} alt="" width={28} height={28} />
        zefix!
      </span>
      <button className="help-button" onClick={onHelp}>
        <HelpIcon size={18} /> Hilfe
      </button>
    </>
  );
}

export function GamePicker({ game, onChange }: { game: GameSetup; onChange: (game: GameSetup) => void }) {
  const suits = gameSuits(game.choice);
  return (
    <span className="game-picker">
      <span className="muted">Spiel</span>
      <Hint>
        Im Sauspiel wählst du die Sau, die du rufst; wer sie hat, spielt mit dir. In Solo, Farbwenz
        und Farbgeier wählst du die Trumpffarbe.
      </Hint>
      <select
        className="select"
        aria-label="Spiel"
        value={game.choice}
        onChange={(e) => onChange(normalizeGame({ ...game, choice: e.target.value as GameChoice }))}
      >
        {GAMES.map((g) => (
          <option key={g.choice} value={g.choice}>
            {g.label}
          </option>
        ))}
      </select>
      {suits.length > 0 && (
        <span className="suit-picker" role="radiogroup" aria-label={suitLabel(game.choice)}>
          {suits.map((suit) => (
            <button
              key={suit}
              role="radio"
              aria-checked={game.suit === suit}
              className={game.suit === suit ? "on" : ""}
              title={`${suitLabel(game.choice)}: ${SUIT_NAMES[suit]}`}
              onClick={() => onChange({ ...game, suit })}
            >
              <SuitIcon suit={suit} />
              <span className="suit-name">{SUIT_NAMES[suit]}</span>
            </button>
          ))}
        </span>
      )}
    </span>
  );
}

export function PositionPicker({ model, onChange }: { model: Model; onChange: (position: Position) => void }) {
  return (
    <span className="labeled">
      <span className="muted">Position</span>
      <Hint>
        Deine Stelle in der Spielreihenfolge. 1 ist Vorhand und spielt als Erstes aus; danach geht es
        im Uhrzeigersinn weiter, also erst zu dem Spieler links von dir.
      </Hint>
      <Segmented
        label="Position"
        value={model.my_position}
        options={POSITIONS.map((p, i) => ({ value: p, label: `${i + 1}` }))}
        onChange={onChange}
      />
    </span>
  );
}

export function PrecisionPicker({ iterations, onChange }: { iterations: number; onChange: (n: number) => void }) {
  return (
    <span className="labeled">
      <span className="muted">Genauigkeit</span>
      <Hint>
        Wie viele passende Blätter gemischt werden. Mehr Blätter, engeres Intervall: Schnell (100.000)
        liegt höchstens etwa 0,3 Prozentpunkte daneben, Normal (1 Mio.) 0,1.
      </Hint>
      <select
        className="select"
        aria-label="Genauigkeit"
        value={iterations}
        onChange={(e) => onChange(Number(e.target.value))}
      >
        {PRECISIONS.map((p) => (
          <option key={p.iterations} value={p.iterations} title={`${p.iterations.toLocaleString("de-DE")} Blätter`}>
            {p.name}
          </option>
        ))}
      </select>
    </span>
  );
}

export function ShareButton({ onClick }: { onClick: () => void }) {
  return (
    <button className="button share-button" onClick={onClick}>
      <ShareIcon size={16} /> Teilen
    </button>
  );
}
