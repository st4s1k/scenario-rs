mod ui;
mod app;
mod lifecycle;
mod shared;

use crate::app::ScenarioApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Scenario Tool",
        options,
        Box::new(|cc| Ok(Box::new(ScenarioApp::new(cc)))),
    )
}
