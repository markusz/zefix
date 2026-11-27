// App state and layout. One layout for every width: CSS stacks the two columns into tabs on
// phones and hides query-language text there (`.code-only`).

import { useEffect, useMemo, useState } from "react";
import { core } from "./core";
import type { Report } from "./core";
import { isDesktop, useSimulation } from "./engine";
import type { Run } from "./engine";
import type { Model } from "./model";
import { Brand, GamePicker, PositionPicker, PrecisionPicker, ShareButton } from "./components/Header";
import { HandIcon, LensIcon } from "./components/Icons";
import { Queries } from "./components/Queries";
import { Seats } from "./components/Seats";
import { ShareDialog } from "./components/Share";
import { EditContext } from "./components/TileRow";
import type { EditState } from "./components/TileRow";
import { format } from "./components/Controls";

/** About ±0,1 percentage points natively; the browser simulates on one core, so ±0,3 there. */
const DEFAULT_ITERATIONS = isDesktop() ? 1_000_000 : 100_000;

type Tab = "seats" | "queries";

/** A shared link wins over the default; a broken one opens the share dialog with the reason. */
function initialModel(): { model: Model; linkError?: string } {
  const linked = core.fromLink(location.hash);
  if (linked === null) return { model: core.defaultModel() };
  if ("ok" in linked) return { model: linked.ok };
  return { model: core.defaultModel(), linkError: linked.err };
}

export function App() {
  const [initial] = useState(initialModel);
  const [model, setModel] = useState(initial.model);
  const [iterations, setIterations] = useState(DEFAULT_ITERATIONS);
  const [edit, setEdit] = useState<EditState>({ selected: null, picker: null });
  const [tab, setTab] = useState<Tab>("seats");
  const [sharing, setSharing] = useState(initial.linkError !== undefined);

  const info = useMemo(() => core.gameInfo(model.game), [model.game]);
  const compiled = useMemo(() => {
    const result = core.compile(model);
    return "ok" in result ? result.ok : undefined;
  }, [model]);
  const run = useSimulation(model, iterations);
  const problem = compiled?.problem ?? null;

  // The address bar mirrors the model: reloading keeps it, copying the address shares it.
  useEffect(() => {
    const fragment = core.linkFragment(model);
    // Replaced, not pushed: every edit would otherwise become a Back step.
    if ("ok" in fragment) history.replaceState(null, "", `#${fragment.ok}`);
  }, [model]);

  if ("err" in info) return <p className="fatal">{info.err}</p>;

  return (
    <EditContext value={{ edit, setEdit }}>
      <div className="app" data-tab={tab}>
        <header className="topbar">
          <div className="brand-strip">
            <Brand />
          </div>
          <div className="controls">
            <GamePicker game={model.game} onChange={(game) => setModel({ ...model, game })} />
            <PositionPicker model={model} onChange={(my_position) => setModel({ ...model, my_position })} />
            <span className="desktop-only">
              <PrecisionPicker iterations={iterations} onChange={setIterations} />
            </span>
            <span className="desktop-only push-right">
              <ShareButton onClick={() => setSharing(true)} />
            </span>
          </div>
        </header>
        <nav className="tabs phone-only" aria-label="Bereiche">
          <button className={tab === "seats" ? "on" : ""} onClick={() => setTab("seats")}>
            <HandIcon className="tab-icon seats-icon" /> Blätter
          </button>
          <button className={tab === "queries" ? "on" : ""} onClick={() => setTab("queries")}>
            <LensIcon className="tab-icon queries-icon" /> Abfragen
          </button>
        </nav>
        <main className="columns">
          <div className="column column-seats">
            <div className="section-head desktop-only">
              <h2 className="section-title" title="Was über die Hände bekannt ist oder angenommen wird. Leere Hände sind zufällig.">
                Blätter
              </h2>
            </div>
            <Seats model={model} setModel={setModel} info={info.ok} compiled={compiled} problem={problem} />
          </div>
          <div className="column column-queries">
            <div className="section-head">
              <h2 className="section-title desktop-only" title="Wie wahrscheinlich ist …">
                Abfragen
              </h2>
              <Status run={run} iterations={iterations} />
            </div>
            <Queries model={model} setModel={setModel} info={info.ok} compiled={compiled} problem={problem} run={run} />
            <div className="phone-only phone-settings">
              <PrecisionPicker iterations={iterations} onChange={setIterations} />
            </div>
          </div>
        </main>
        <ShareDialog
          open={sharing}
          model={model}
          onClose={() => setSharing(false)}
          onImport={(m) => {
            setModel(m);
            setEdit({ selected: null, picker: null });
          }}
          initialMessage={initial.linkError}
        />
      </div>
    </EditContext>
  );
}

function Status({ run, iterations }: { run: Run; iterations: number }) {
  const { status } = run;
  switch (status.kind) {
    case "running":
      return (
        <div className="status">
          <div className="progress" role="progressbar" aria-valuenow={Math.round(status.fraction * 100)}>
            <div style={{ width: `${status.fraction * 100}%` }} />
          </div>
          <button className="link-button desktop-only" onClick={run.stop}>
            Stopp
          </button>
        </div>
      );
    case "finished":
      return run.report ? <p className="status muted small-text desktop-only">{summary(run.report, iterations)}</p> : null;
    case "stopped":
      return (
        <p className="status muted small-text">
          Angehalten.{" "}
          <button className="link-button" onClick={run.restart}>
            Neu starten
          </button>
        </p>
      );
    case "failed":
      return <p className="status error-text">{status.message}</p>;
    case "invalid": {
      const p = status.problem;
      const text =
        p.kind === "noQueries"
          ? "Füge eine Abfrage mit mindestens einem Baustein hinzu."
          : p.kind === "filter"
            ? "Die weitere Bedingung enthält einen Fehler."
            : p.kind === "query"
              ? "Eine Abfrage enthält einen Fehler."
              : p.message;
      return <p className="status error-text">{text}</p>;
    }
  }
}

function summary(report: Report, iterations: number): string {
  let text = `${format.int(report.accepted)} von ${format.int(iterations)} Blättern gemischt`;
  if (report.attempts !== report.accepted) {
    text += ` · davon passend ${format.percent(report.accepted / report.attempts)}`;
  }
  return text;
}
