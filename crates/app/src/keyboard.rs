//! iOS opens the on-screen keyboard only for an input focused inside a touch handler. eframe
//! focuses its hidden text input a frame after the tap, so on an iPhone tapping a text field shows
//! no keyboard. Text fields register their rect here; a tap that ends on one focuses eframe's
//! input right away, still inside the touch handler.

#[cfg(target_arch = "wasm32")]
pub use web::{begin_frame, install, text_field};

#[cfg(not(target_arch = "wasm32"))]
pub fn begin_frame(_ctx: &eframe::egui::Context) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn text_field(_response: &eframe::egui::Response) {}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::egui::{Context, Pos2, Rect, Response, pos2};
    use eframe::wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use std::cell::RefCell;

    /// How far (CSS px) a finger may move and still tap rather than scroll.
    const TAP_SLOP: f32 = 10.0;

    struct Fields {
        rects: Vec<Rect>,
        zoom: f32,
        touch_start: Option<Pos2>,
    }

    thread_local! {
        static FIELDS: RefCell<Fields> = const {
            RefCell::new(Fields {
                rects: Vec::new(),
                zoom: 1.0,
                touch_start: None,
            })
        };
    }

    pub fn begin_frame(ctx: &Context) {
        FIELDS.with_borrow_mut(|f| {
            f.rects.clear();
            f.zoom = ctx.zoom_factor();
        });
    }

    pub fn text_field(response: &Response) {
        FIELDS.with_borrow_mut(|f| f.rects.push(response.rect));
    }

    pub fn install(canvas: &web_sys::HtmlCanvasElement) -> Result<(), JsValue> {
        let start = Closure::<dyn FnMut(web_sys::TouchEvent)>::new(|event: web_sys::TouchEvent| {
            let start = first_touch(&event.touches());
            FIELDS.with_borrow_mut(|f| f.touch_start = start);
        });
        canvas.add_event_listener_with_callback("touchstart", start.as_ref().unchecked_ref())?;
        start.forget();

        let target = canvas.clone();
        let end =
            Closure::<dyn FnMut(web_sys::TouchEvent)>::new(move |event: web_sys::TouchEvent| {
                if let Some(end) = first_touch(&event.changed_touches())
                    && ends_on_field(&target, end)
                {
                    focus_text_agent(&target);
                }
            });
        canvas.add_event_listener_with_callback("touchend", end.as_ref().unchecked_ref())?;
        end.forget();
        Ok(())
    }

    fn first_touch(touches: &web_sys::TouchList) -> Option<Pos2> {
        let touch = touches.get(0)?;
        Some(pos2(touch.client_x() as f32, touch.client_y() as f32))
    }

    /// `end` is in CSS px; fields are in egui points relative to the canvas.
    fn ends_on_field(canvas: &web_sys::HtmlCanvasElement, end: Pos2) -> bool {
        let origin = canvas.get_bounding_client_rect();
        FIELDS.with_borrow(|f| {
            let is_tap = f.touch_start.is_some_and(|s| s.distance(end) <= TAP_SLOP);
            let pos = pos2(
                (end.x - origin.left() as f32) / f.zoom,
                (end.y - origin.top() as f32) / f.zoom,
            );
            is_tap && f.rects.iter().any(|r| r.contains(pos))
        })
    }

    /// eframe places its text input next to the canvas.
    fn focus_text_agent(canvas: &web_sys::HtmlCanvasElement) {
        let input = canvas
            .parent_element()
            .and_then(|parent| parent.query_selector("input").ok().flatten())
            .and_then(|input| input.dyn_into::<web_sys::HtmlElement>().ok());
        if let Some(input) = input {
            let options = web_sys::FocusOptions::new();
            options.set_prevent_scroll(true);
            // Failing leaves the old behaviour: the keyboard opens on the second tap.
            input.focus_with_options(&options).ok();
        }
    }
}
