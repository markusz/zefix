// Where simulations run. In the browser a Web Worker steps the WebAssembly build; in the
// desktop app (Tauri) the native core runs on all cores. Both answer with `EngineMessage`s.

import { useEffect, useRef, useState } from "react";
import type { Problem, Report } from "./core";
import type { Model } from "./model";

export type WorkerRequest =
  | { kind: "start"; id: number; model: string; iterations: number }
  | { kind: "stop" };

export type EngineMessage =
  | { kind: "report"; id: number; report: Report }
  | { kind: "invalid"; id: number; problem: Problem }
  | { kind: "failed"; id: number; message: string };

export type RunStatus =
  | { kind: "running"; fraction: number }
  | { kind: "finished" }
  | { kind: "stopped" }
  | { kind: "invalid"; problem: Problem }
  | { kind: "failed"; message: string };

export interface Run {
  status: RunStatus;
  /** Latest numbers; kept while a new run starts, so they do not flicker. */
  report: Report | undefined;
  /** The report belongs to an older model than the one on screen. */
  stale: boolean;
  stop: () => void;
  restart: () => void;
}

interface Backend {
  start(id: number, model: string, iterations: number): void;
  stop(): void;
  dispose(): void;
}

export function isDesktop(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function workerBackend(onMessage: (message: EngineMessage) => void): Backend {
  const worker = new Worker(new URL("./sim.worker.ts", import.meta.url), { type: "module" });
  worker.onmessage = (event: MessageEvent<EngineMessage>) => onMessage(event.data);
  return {
    start: (id, model, iterations) =>
      worker.postMessage({ kind: "start", id, model, iterations } satisfies WorkerRequest),
    stop: () => worker.postMessage({ kind: "stop" } satisfies WorkerRequest),
    dispose: () => worker.terminate(),
  };
}

function desktopBackend(onMessage: (message: EngineMessage) => void): Backend {
  // Loaded only in the app, so the web build does not carry it.
  const tauri = import("@tauri-apps/api/core");
  return {
    start: (id, model, iterations) => {
      void tauri.then(({ Channel, invoke }) => {
        const channel = new Channel<EngineMessage>();
        channel.onmessage = onMessage;
        return invoke("simulate", { model, iterations, id, channel });
      });
    },
    stop: () => void tauri.then(({ invoke }) => invoke("stop")),
    dispose: () => void tauri.then(({ invoke }) => invoke("stop")),
  };
}

/** Restarts whenever the model or the run length changes. */
export function useSimulation(model: Model, iterations: number): Run {
  const backend = useRef<Backend | undefined>(undefined);
  const runId = useRef(0);
  /** Set by `stop`; a report already on its way must not flip the status back to running. */
  const stopped = useRef(false);
  const [status, setStatus] = useState<RunStatus>({ kind: "stopped" });
  const [shown, setShown] = useState<{ id: number; report: Report } | undefined>();
  const [restarts, setRestarts] = useState(0);

  useEffect(() => {
    const onMessage = (message: EngineMessage) => {
      if (message.id !== runId.current || stopped.current) return;
      switch (message.kind) {
        case "report":
          setShown({ id: message.id, report: message.report });
          setStatus(
            message.report.finished
              ? { kind: "finished" }
              : { kind: "running", fraction: message.report.fraction },
          );
          break;
        case "invalid":
          setStatus({ kind: "invalid", problem: message.problem });
          break;
        case "failed":
          setStatus({ kind: "failed", message: message.message });
          break;
      }
    };
    const b = isDesktop() ? desktopBackend(onMessage) : workerBackend(onMessage);
    backend.current = b;
    return () => b.dispose();
  }, []);

  const json = JSON.stringify(model);
  useEffect(() => {
    const id = ++runId.current;
    stopped.current = false;
    setStatus({ kind: "running", fraction: 0 });
    backend.current?.start(id, json, iterations);
  }, [json, iterations, restarts]);

  return {
    status,
    report: shown?.report,
    stale: shown !== undefined && (shown.id !== runId.current || status.kind === "invalid"),
    stop: () => {
      stopped.current = true;
      backend.current?.stop();
      setStatus({ kind: "stopped" });
    },
    restart: () => setRestarts((n) => n + 1),
  };
}
