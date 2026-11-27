// The questions asked of every deal, each with its probability right inside its card.

import { useContext } from "react";
import type { Compiled, GameInfo, Problem, QueryResult } from "../core";
import type { Run } from "../engine";
import type { Model, QueryModel, Scope } from "../model";
import {
  OTHER_PLAYERS,
  POSITIONS,
  SEATS,
  fitScope,
  positionOf,
  scopeKey,
  scopeLabel,
  scopeOptions,
  seatName,
  visualQuery,
} from "../model";
import { CardFace } from "./Cards";
import { Segmented, Stepper, format } from "./Controls";
import { PersonIcon, PlusIcon, TrashIcon } from "./Icons";
import { EditContext, TileRow } from "./TileRow";

interface Props {
  model: Model;
  setModel: (model: Model) => void;
  info: GameInfo;
  compiled: Compiled | undefined;
  problem: Problem | null;
  run: Run;
}

export function Queries({ model, setModel, info, compiled, problem, run }: Props) {
  const { edit, setEdit } = useContext(EditContext);
  const setQuery = (index: number, query: QueryModel) =>
    setModel({ ...model, queries: model.queries.map((q, i) => (i === index ? query : q)) });
  const remove = (index: number) => {
    setModel({ ...model, queries: model.queries.filter((_, i) => i !== index) });
    // Later queries shift, so selections inside queries would point elsewhere.
    setEdit({
      selected: edit.selected?.row.startsWith("query:") ? null : edit.selected,
      picker: edit.picker?.startsWith("query:") ? null : edit.picker,
    });
  };
  const add = () =>
    setModel({
      ...model,
      queries: [...model.queries, visualQuery(`Abfrage ${model.queries.length + 1}`, "AnyOther", [])],
    });
  return (
    <section className="queries" aria-label="Abfragen">
      {model.queries.map((query, index) => (
        <QueryCard
          key={index}
          index={index}
          query={query}
          model={model}
          info={info}
          expr={compiled?.queries[index]?.expr ?? null}
          error={problem?.kind === "query" && problem.name === query.name ? problem.rendered : null}
          result={run.report?.results.find((r) => r.name === query.name)}
          stale={run.stale}
          onChange={(q) => setQuery(index, q)}
          onRemove={() => remove(index)}
        />
      ))}
      <button className="button add-query" onClick={add}>
        <PlusIcon size={16} /> Abfrage
      </button>
    </section>
  );
}

interface CardProps {
  index: number;
  query: QueryModel;
  model: Model;
  info: GameInfo;
  expr: string | null;
  error: string | null;
  result: QueryResult | undefined;
  stale: boolean;
  onChange: (query: QueryModel) => void;
  onRemove: () => void;
}

function QueryCard({ index, query, model, info, expr, error, result, stale, onChange, onRemove }: CardProps) {
  const sauspiel = info.called_sau !== null;
  const scope = fitScope(query.scope, sauspiel);
  return (
    <article className="card query accent-queries">
      <header className="query-head">
        <input
          className="query-name"
          value={query.name}
          aria-label="Name der Abfrage"
          onChange={(e) => onChange({ ...query, name: e.target.value })}
        />
        <span className="code-only">
          <Segmented
            label="Eingabe"
            value={query.mode}
            options={[
              { value: "Visual", label: "Bausteine" },
              { value: "Text", label: "Text" },
            ]}
            // Switching to text starts from the generated expression, so nothing is lost.
            onChange={(mode) =>
              onChange({ ...query, mode, text: mode === "Text" && !query.text ? (expr ?? "") : query.text })
            }
          />
        </span>
        <button className="icon-button" onClick={onRemove} title={`Abfrage ${index + 1} löschen`}>
          <TrashIcon />
        </button>
      </header>
      {query.mode === "Visual" ? (
        <>
          <ScopePicker scope={scope} sauspiel={sauspiel} model={model} onChange={(s) => onChange({ ...query, scope: s })} />
          <TileRow
            row={`query:${index}`}
            tiles={query.tiles}
            onChange={(tiles) => onChange({ ...query, tiles })}
            info={info}
            accent="queries"
          />
          {expr ? (
            <pre className="code-box code-only">{expr}</pre>
          ) : (
            <p className="muted small-text">Noch leer: Karten oder eine Regel hinzufügen.</p>
          )}
        </>
      ) : (
        <textarea
          className="code"
          rows={2}
          spellCheck={false}
          value={query.text}
          onChange={(e) => onChange({ ...query, text: e.target.value })}
        />
      )}
      {error && <pre className="error-box">{error}</pre>}
      {result && <Result result={result} stale={stale} model={model} />}
    </article>
  );
}

function ScopePicker({
  scope,
  sauspiel,
  model,
  onChange,
}: {
  scope: Scope;
  sauspiel: boolean;
  model: Model;
  onChange: (scope: Scope) => void;
}) {
  const options = scopeOptions(sauspiel, model.my_position);
  return (
    <div className="scope">
      <PersonIcon className="muted" />
      <select
        className="select"
        aria-label="Wer"
        value={scopeKey(scope)}
        onChange={(e) => onChange(options.find((o) => scopeKey(o) === e.target.value) ?? scope)}
      >
        {options.map((o) => (
          <option key={scopeKey(o)} value={scopeKey(o)}>
            {scopeKey(o) === "AtLeast" ? "mehrere andere Spieler" : scopeLabel(o)}
          </option>
        ))}
      </select>
      {typeof scope !== "string" && "AtLeast" in scope && (
        <Stepper
          value={scope.AtLeast}
          min={1}
          max={OTHER_PLAYERS}
          onChange={(n) => onChange({ AtLeast: n })}
          prefix="mind. "
        />
      )}
    </div>
  );
}

function Result({ result, stale, model }: { result: QueryResult; stale: boolean; model: Model }) {
  return (
    <footer className={`result ${stale ? "stale" : ""}`}>
      <div className="result-row">
        <span className="result-detail muted small-text">
          ± {format.points(result.margin)} Punkte · {format.int(result.hits)} Treffer
        </span>
        <span className="percent">{format.percent(result.p)}</span>
      </div>
      {result.examples.length > 0 && (
        <details className="examples code-only">
          <summary>Beispiele</summary>
          {result.examples.map((deal, i) => (
            <div key={i} className="deal">
              {SEATS.map((seat, s) => ({ seat, s, position: positionOf(seat, model.my_position) }))
                .sort((a, b) => POSITIONS.indexOf(a.position) - POSITIONS.indexOf(b.position))
                .map(({ seat, s }) => (
                  <div key={seat} className="deal-hand">
                    <span className="deal-seat">
                      {seatName(seat)}
                      {deal.partner === seat && <span className="muted"> · Partner</span>}
                    </span>
                    <span className="deal-cards">
                      {(deal.hands[s] ?? []).map((card) => (
                        <span key={card} className="mini">
                          <CardFace card={card} />
                        </span>
                      ))}
                    </span>
                  </div>
                ))}
            </div>
          ))}
        </details>
      )}
    </footer>
  );
}
