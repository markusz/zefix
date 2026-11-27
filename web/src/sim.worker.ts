// Runs the simulation off the main thread. Steps in time slices, reports after each, and
// yields between slices so a new model can cancel the run.

import init, { Simulation } from "./wasm/zefix_wasm.js";
import type { EngineMessage, WorkerRequest } from "./engine";

/** Simulation time between reports; short enough for live numbers, long enough to be cheap. */
const SLICE_MS = 60;

const loaded = init();
let current: { id: number; sim: Simulation } | undefined;

function post(message: EngineMessage) {
  self.postMessage(message);
}

function stop() {
  current?.sim.free();
  current = undefined;
}

function slice(id: number) {
  if (current?.id !== id) return;
  const { sim } = current;
  const deadline = performance.now() + SLICE_MS;
  try {
    let finished = false;
    while (!finished && performance.now() < deadline) finished = sim.advance(1);
    post({ kind: "report", id, report: JSON.parse(sim.report()) });
    if (finished) stop();
    else setTimeout(() => slice(id), 0);
  } catch (error) {
    stop();
    post({ kind: "failed", id, message: String(error) });
  }
}

self.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  await loaded;
  const request = event.data;
  stop();
  if (request.kind === "stop") return;
  try {
    current = { id: request.id, sim: Simulation.start(request.model, request.iterations) };
  } catch (problem) {
    post({ kind: "invalid", id: request.id, problem: JSON.parse(String(problem)) });
    return;
  }
  slice(request.id);
};
