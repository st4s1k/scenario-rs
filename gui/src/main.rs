mod ui;
mod app;
mod lifecycle;
mod shared;

use crate::app::ScenarioApp;

const APP_STATE_FILE: &'static str = "scenario-rs-state.json";

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Scenario Tool",
        options,
        Box::new(|_cc| Ok(Box::new(ScenarioApp::load(APP_STATE_FILE)))),
    )
}
