// One row per player in play order, holding what is known or assumed about that hand.

import type { Compiled, GameInfo, Problem } from "../core";
import type { Model, Tile } from "../model";
import { POSITIONS, SEATS, cardsElsewhere, seatAt, seatName } from "../model";
import { Hint } from "./Hint";
import { TileRow } from "./TileRow";

interface Props {
  model: Model;
  setModel: (model: Model) => void;
  info: GameInfo;
  compiled: Compiled | undefined;
  problem: Problem | null;
}

export function Seats({ model, setModel, info, compiled, problem }: Props) {
  const setTiles = (index: number, tiles: Tile[]) =>
    setModel({ ...model, seats: model.seats.map((s, i) => (i === index ? { tiles } : s)) });
  const filterError = problem?.kind === "filter" ? problem.rendered : null;
  const hasCondition = model.filter_text.trim() !== "" || filterError !== null;
  return (
    <section className="seats" aria-label="Blätter">
      {POSITIONS.map((position, i) => {
        const seat = seatAt(position, model.my_position);
        const index = SEATS.indexOf(seat);
        return (
          <div key={seat} className={`card seat ${seat === "me" ? "accent-me" : "accent-seats"}`}>
            <div className="seat-label">
              {seatName(seat)}
              {i === 0 && <span className="muted"> · Vorhand</span>}
            </div>
            <TileRow
              row={`seat:${seat}`}
              tiles={model.seats[index]?.tiles ?? []}
              onChange={(tiles) => setTiles(index, tiles)}
              info={info}
              blocked={cardsElsewhere(model, seat)}
              accent={seat === "me" ? "me" : "seats"}
            />
          </div>
        );
      })}
      <details className={`extra ${hasCondition ? "" : "code-only"}`} open={hasCondition || undefined}>
        <summary>
          Weitere Bedingung{" "}
          <Hint>
            Für alles, was Karten und Regeln nicht ausdrücken, etwa ein „oder“, in der Abfragesprache.
            Nur Blätter, die sie erfüllen, zählen.
          </Hint>
        </summary>
        <textarea
          className="code"
          rows={2}
          value={model.filter_text}
          placeholder="z. B. left.frei(gras) or right.has(SO)"
          spellCheck={false}
          onChange={(e) => setModel({ ...model, filter_text: e.target.value })}
        />
        {filterError && <pre className="error-box">{filterError}</pre>}
      </details>
      {compiled && (
        <details className="extra code-only">
          <summary>Als Abfragetext</summary>
          <pre className="code-box">
            {`Feste Karten: [${compiled.fixed.join(", ")}]\nFilter: ${compiled.filter ?? "–"}`}
          </pre>
        </details>
      )}
    </section>
  );
}
