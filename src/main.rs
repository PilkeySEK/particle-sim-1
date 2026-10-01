use crate::app::App;

mod app;
mod math;
mod universe;
mod util;

#[cfg_attr(target_arch = "wasm32", tokio::main(flavor = "current_thread"))]
#[cfg_attr(not(target_arch = "wasm32"), tokio::main)]
async fn main() -> color_eyre::Result {
    tracing_subscriber::fmt().init();

    #[cfg(not(target_arch = "wasm32"))]
    return run_native().await;
    #[cfg(target_arch = "wasm32")]
    return run_web().await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_native() -> color_eyre::Result {
    use eframe::NativeOptions;

    eframe::run_native(
        "Particle Sim 3",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn run_web() -> color_eyre::Result {
    use eframe::{WebOptions, wasm_bindgen::JsCast};

    let web_options = WebOptions::default();

    let document = web_sys::window()
        .expect("No window")
        .document()
        .expect("No document");

    let canvas = document
        .get_element_by_id("the_canvas_id")
        .expect("Failed to find the_canvas_id")
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .expect("the_canvas_id was not a HtmlCanvasElement");

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|_cc| Ok(Box::new(App::new()))),
        )
        .await
        .unwrap();

    Ok(())
}
