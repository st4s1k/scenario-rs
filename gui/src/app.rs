use crate::{
    lifecycle::{execution_lifecycle, LifecycleHandler},
    shared::SEPARATOR,
    ui::MyUi,
};
use egui::Context;
use egui_file_dialog::FileDialog;
use scenario_rs::{
    config::ScenarioConfig,
    scenario::Scenario,
};
use serde::{Deserialize, Serialize};
use std::ops::DerefMut;
use std::{
    path::PathBuf,
    sync::mpsc,
};

#[derive(Serialize, Deserialize, Default)]
pub struct ScenarioApp {
    config_path: Option<PathBuf>,
    output_log: String,

    #[serde(skip)]
    is_executing: bool,

    #[serde(skip)]
    config_file_dialog: FileDialog,

    #[serde(skip)]
    receiver: Option<mpsc::Receiver<String>>,

    #[serde(skip)]
    state_file: String,

    #[serde(skip)]
    scenario: Option<Scenario>,
}

impl ScenarioApp {
    pub fn load(state_file_path: &str) -> Self {
        if let Ok(json) = std::fs::read_to_string(state_file_path) {
            if let Ok(mut app_state) = serde_json::from_str::<ScenarioApp>(&json) {
                app_state.state_file = state_file_path.to_string();
                return app_state;
            }
        }
        ScenarioApp::default()
    }

    pub fn save_state(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(self.state_file.as_str(), json);
        }
    }

    pub fn start_scenario(&mut self) {
        if !self.is_executing {
            let (tx, rx) = mpsc::channel();
            self.receiver = Some(rx);
            self.is_executing = true;
            self.run_scenario(tx.clone());
        }
    }

    fn run_scenario(&mut self, tx: mpsc::Sender<String>) {
        LifecycleHandler::try_initialize(tx.clone());

        if let Err(e) = self.execute_scenario(&tx) {
            let mut log_message = String::new();
            log_message.push_str(&format!("{SEPARATOR}\n"));
            log_message.push_str(&format!("Scenario failed: {e}\n"));
            log_message.push_str(&format!("{SEPARATOR}\n"));
            let _ = tx.send(log_message);
        }

        self.is_executing = false;
    }

    fn execute_scenario(&mut self, tx: &mpsc::Sender<String>) -> Result<(), String> {
        let mut log_message = String::new();

        log_message.push_str(&format!("{SEPARATOR}\n"));

        let lifecycle = execution_lifecycle();

        self.scenario.as_ref().unwrap()
            .execute_with_lifecycle(lifecycle)
            .map_err(|e| e.to_string())?;

        log_message.push_str(&format!("{SEPARATOR}\n"));
        log_message.push_str("Scenario completed successfully!\n");
        log_message.push_str(&format!("{SEPARATOR}\n"));

        let _ = tx.send(log_message);

        Ok(())
    }

    pub fn handle_incoming_logs(&mut self, ctx: &Context) {
        if let Some(ref rx) = self.receiver {
            while let Ok(msg) = rx.try_recv() {
                self.output_log.push_str(&msg);
            }

            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    pub fn update_file_dialogs(&mut self, ctx: &Context) {
        self.config_file_dialog.update(ctx);
        if let Some(path) = self.config_file_dialog.take_selected() {
            self.config_path = Some(path);
        }
    }
}

impl eframe::App for ScenarioApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        catppuccin_egui::set_theme(&ctx, catppuccin_egui::FRAPPE);
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Grid::new("scenario_tool_grid")
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.set_width(600.0);

                    ui.heading("Scenario Tool");
                    ui.end_row();
                    ui.file_selector_field(
                        "Config Path:", self.config_path.as_ref(),
                        "Select Config File", &mut self.config_file_dialog,
                    );
                    match &mut self.scenario {
                        Some(scenario) => {
                            ui.heading("Required variables:");
                            ui.end_row();
                            for required_variable in scenario.variables().required().deref_mut() {
                                let label = required_variable.label().to_owned();
                                ui.mutable_labeled_field(&label, required_variable.value());
                            }
                            ui.heading("Defined variables:");
                            ui.end_row();
                            for (name, value) in scenario.variables().defined().unwrap() {
                                if name == "password" {
                                    continue;
                                }
                                ui.labeled_field(&name, &value);
                            }
                            if ui.button("Execute").clicked() {
                                self.start_scenario();
                            }
                        }
                        None => if let Some(config_path) = self.config_path.clone() {
                            match ScenarioConfig::try_from(config_path) {
                                Ok(config) => {
                                    let scenario = Scenario::new(config)
                                        .map_err(|error| error.to_string()).unwrap();
                                    self.scenario = Some(scenario);
                                }
                                Err(error) => {
                                    self.output_log.clear();
                                    self.output_log.push_str(&format!("{SEPARATOR}\n"));
                                    self.output_log.push_str(&format!("Error: {error}\n"));
                                    self.output_log.push_str(&format!("{SEPARATOR}\n"));
                                }
                            };
                        }
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
