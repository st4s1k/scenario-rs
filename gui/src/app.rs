use crate::{
    lifecycle::{execution_lifecycle, LifecycleHandler},
    shared::SEPARATOR,
    ui::MyUi,
};
use colored::Colorize;
use egui::Context;
use egui_file_dialog::FileDialog;
use scenario_rs::{
    config::ScenarioConfig,
    scenario::Scenario,
};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::mpsc, thread};

#[derive(Serialize, Deserialize, Default)]
pub struct ScenarioApp {
    pub config_path: Option<PathBuf>,
    pub output_log: String,

    #[serde(skip)]
    pub is_executing: bool,

    #[serde(skip)]
    pub config_file_dialog: FileDialog,

    #[serde(skip)]
    pub receiver: Option<mpsc::Receiver<String>>,
}

impl Clone for ScenarioApp {
    fn clone(&self) -> Self {
        Self {
            config_path: self.config_path.clone(),
            output_log: self.output_log.clone(),
            is_executing: self.is_executing,
            config_file_dialog: FileDialog::new(),
            receiver: None,
        }
    }
}

impl ScenarioApp {
    const APP_STATE_FILE: &'static str = "scenario-rs-state.json";

    pub fn new(_cc: &eframe::CreationContext) -> Self {
        ScenarioApp::load()
    }

    pub fn load() -> Self {
        if let Ok(json) = std::fs::read_to_string(Self::APP_STATE_FILE) {
            if let Ok(app_state) = serde_json::from_str(&json) {
                return app_state;
            }
        }
        ScenarioApp::default()
    }

    pub fn save_state(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::APP_STATE_FILE, json);
        }
    }

    pub fn start_scenario(&mut self) {
        if !self.is_executing {
            self.is_executing = true;
            let (tx, rx) = mpsc::channel();

            let app_state = ScenarioApp {
                config_path: self.config_path.clone(),
                output_log: self.output_log.clone(),
                is_executing: false,
                config_file_dialog: FileDialog::new(),
                receiver: None,
            };

            thread::spawn(move || {
                app_state.run_scenario(tx.clone());
                let _ = tx.send("SCENARIO_FINISHED".to_string());
            });

            self.receiver = Some(rx);
        }
    }

    fn run_scenario(&self, tx: mpsc::Sender<String>) {
        LifecycleHandler::try_initialize(tx.clone());

        if let Err(e) = self.execute_scenario(&tx) {
            let mut log_message = String::new();
            log_message.push_str(&format!("{SEPARATOR}\n"));
            log_message.push_str(&format!("Scenario failed: {}\n", e));
            log_message.push_str(&format!("{SEPARATOR}\n"));
            let _ = tx.send(log_message);
        }
    }

    fn execute_scenario(&self, tx: &mpsc::Sender<String>) -> Result<(), String> {
        let mut log_message = String::new();

        log_message.push_str(&format!("{SEPARATOR}\n"));

        let config = ScenarioConfig::try_from(self.config_path.clone().ok_or_else(|| "No config file selected".to_string())?)
            .map_err(|e| e.to_string())?;

        let scenario = Scenario::new(config)
            .map_err(|e| e.to_string())?;

        let lifecycle = execution_lifecycle();
        scenario.execute_with_lifecycle(lifecycle)
            .map_err(|e| e.to_string())?;

        log_message.push_str(&format!("{}\n", SEPARATOR));
        log_message.push_str(&format!("{}\n", "Scenario completed successfully!".cyan()));
        log_message.push_str(&format!("{}\n", SEPARATOR));
        let _ = tx.send(log_message);

        Ok(())
    }

    pub fn handle_incoming_logs(&mut self, ctx: &Context) {
        if let Some(ref rx) = self.receiver {
            while let Ok(msg) = rx.try_recv() {
                if msg == "SCENARIO_FINISHED" {
                    self.is_executing = false;
                } else {
                    self.output_log.push_str(&msg);
                }
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
            ui.heading("Scenario Tool");
            egui::Grid::new("scenario_tool_grid")
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    // TODO: Take all the variables and display them as fields
                    // ui.labeled_field("Service Name:", &mut self);
                    // ui.labeled_field("Username:", &mut self.username);
                    // ui.labeled_field("Host:", &mut self.host);
                    // ui.labeled_field("Port:", &mut self.port);
                    ui.file_selector_field(
                        "Config Path:", self.config_path.as_ref(),
                        "Select Config File", &mut self.config_file_dialog,
                    );
                    // TODO: Find a way to indicate that the variable is a path
                    // ui.file_selector_field(
                    //     "JAR Path:", self.jar_path.as_ref(),
                    //     "Select Config File", &mut self.jar_file_dialog,
                    // );
                    if ui.button("Execute").clicked() {
                        self.start_scenario();
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
