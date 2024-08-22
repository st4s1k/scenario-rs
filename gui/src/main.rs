mod ui;
mod app;
mod scenario;
mod lifecycle;
mod shared;

use crate::{
    app::ScenarioApp,
    scenario::start_scenario,
    ui::MyUi,
};
use eframe::egui;
use egui::Context;

impl eframe::App for ScenarioApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        catppuccin_egui::set_theme(&ctx, catppuccin_egui::FRAPPE);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Scenario Tool");
            egui::Grid::new("scenario_tool_grid")
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.labeled_field("Service Name:", &mut self.service_name);
                    ui.labeled_field("Username:", &mut self.username);
                    ui.password_field("Password:", &mut self.password);
                    ui.labeled_field("Host:", &mut self.host);
                    ui.labeled_field("Port:", &mut self.port);
                    ui.file_selector_field(
                        "Config Path:", self.config_path.as_ref(),
                        "Select Config File", &mut self.config_file_dialog,
                    );
                    ui.file_selector_field(
                        "JAR Path:", self.jar_path.as_ref(),
                        "Select Config File", &mut self.jar_file_dialog,
                    );
                    if ui.button("Execute").clicked() {
                        start_scenario(self);
                    }
                });

            self.handle_incoming_logs(ctx);

            ui.separator();
            ui.text_area("Scenario Log:", &mut self.output_log);

            self.update_file_dialogs(ctx);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_state();
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Scenario Tool",
        options,
        Box::new(|cc| Ok(Box::new(ScenarioApp::new(cc)))),
    )
}
