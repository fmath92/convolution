mod app;

pub use app::ConvolutionApp;

#[cfg(target_arch = "wasm32")]
pub fn main() {
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;

    console_error_panic_hook::set_once();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .expect("No browser document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Canvas not found")
            .dyn_into::<HtmlCanvasElement>()
            .expect("Element is not a canvas");

        let web_options = eframe::WebOptions::default();
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(ConvolutionApp::new(cc)))),
            )
            .await;

        if let Err(err) = result {
            eprintln!("Failed to start eframe app: {err:?}");
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "WASM Convolution Explorer",
        native_options,
        Box::new(|cc| Ok(Box::new(ConvolutionApp::new(cc)))),
    )
}
