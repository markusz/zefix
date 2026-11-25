//! Desktop and web UI for schafsim: build a scenario from card tiles and watch the probabilities converge.

mod app;
mod format;
mod model;
mod paint;
mod runner;
mod theme;
mod view;

const TITLE: &str = "Schafsim";

#[cfg(not(target_arch = "wasm32"))]
const WINDOW_SIZE: [f32; 2] = [1440.0, 960.0];
#[cfg(not(target_arch = "wasm32"))]
const MIN_WINDOW_SIZE: [f32; 2] = [900.0, 600.0];

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(TITLE)
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        TITLE,
        options,
        Box::new(|cc| Ok(Box::new(app::SchafsimApp::new(&cc.egui_ctx)))),
    )
}

/// Element ids in `index.html`.
#[cfg(target_arch = "wasm32")]
const CANVAS_ID: &str = "schafsim_canvas";
#[cfg(target_arch = "wasm32")]
const LOADING_ID: &str = "loading";

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(message) = start_web().await {
            show_loading_text(&format!("{TITLE} konnte nicht starten: {message}"));
        }
    });
}

#[cfg(target_arch = "wasm32")]
async fn start_web() -> Result<(), String> {
    use eframe::wasm_bindgen::JsCast as _;
    let canvas = document()
        .and_then(|d| d.get_element_by_id(CANVAS_ID))
        .ok_or("Zeichenfläche fehlt")?
        .dyn_into::<eframe::web_sys::HtmlCanvasElement>()
        .map_err(|_| "Zeichenfläche ist kein Canvas")?;
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(app::SchafsimApp::new(&cc.egui_ctx)))),
        )
        .await
        .map_err(|e| format!("{e:?}"))?;
    if let Some(loading) = document().and_then(|d| d.get_element_by_id(LOADING_ID)) {
        loading.remove();
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn document() -> Option<eframe::web_sys::Document> {
    eframe::web_sys::window().and_then(|w| w.document())
}

#[cfg(target_arch = "wasm32")]
fn show_loading_text(text: &str) {
    if let Some(loading) = document().and_then(|d| d.get_element_by_id(LOADING_ID)) {
        loading.set_text_content(Some(text));
    }
}
