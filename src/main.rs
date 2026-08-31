use eframe::NativeOptions;

use crate::app::App;

mod app;
mod math;
mod universe;

#[tokio::main]
async fn main() -> color_eyre::Result {
    tracing_subscriber::fmt().init();

    eframe::run_native(
        "Particle Sim 3",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )?;
    Ok(())
}
