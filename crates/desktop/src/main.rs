//! zefix! as a Mac app: the web UI in a native window. The UI keeps using its WebAssembly core
//! for everything quick (compiling, share links); only simulations run here, natively on all
//! cores, and stream their progress back in the shape the browser's worker uses.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::ipc::Channel;
use zefix_model::api;

/// Chunks per step: enough to keep every core busy, few enough for several reports a second.
const CHUNKS_PER_STEP: u64 = 32;

/// The run that may report; starting or stopping another makes older runs quit.
#[derive(Default)]
struct Current(Arc<AtomicU64>);

/// Starts a simulation and streams `EngineMessage`s (see `web/src/engine.ts`) for run `id`.
#[tauri::command]
fn simulate(
    model: String,
    iterations: u64,
    id: u64,
    channel: Channel<Value>,
    current: tauri::State<'_, Current>,
) {
    let current = Arc::clone(&current.0);
    current.store(id, Ordering::SeqCst);
    std::thread::spawn(move || {
        let mut sim = match api::start(&model, iterations) {
            Ok(sim) => sim,
            Err(problem) => {
                let problem: Value = serde_json::from_str(&problem).unwrap_or(Value::Null);
                let _ = channel.send(json!({ "kind": "invalid", "id": id, "problem": problem }));
                return;
            }
        };
        while current.load(Ordering::SeqCst) == id {
            match sim.advance(CHUNKS_PER_STEP) {
                Ok(finished) => {
                    let report: Value = serde_json::to_value(sim.report()).unwrap_or(Value::Null);
                    let _ = channel.send(json!({ "kind": "report", "id": id, "report": report }));
                    if finished {
                        return;
                    }
                }
                Err(message) => {
                    let _ = channel.send(json!({ "kind": "failed", "id": id, "message": message }));
                    return;
                }
            }
        }
    });
}

#[tauri::command]
fn stop(current: tauri::State<'_, Current>) {
    current.0.fetch_add(1, Ordering::SeqCst);
}

fn main() {
    let app = tauri::Builder::default()
        .manage(Current::default())
        .invoke_handler(tauri::generate_handler![simulate, stop])
        .run(tauri::generate_context!());
    if let Err(e) = app {
        eprintln!("zefix! konnte nicht starten: {e}");
        std::process::exit(1);
    }
}
