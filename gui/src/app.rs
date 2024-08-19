use egui::Context;
use egui_file_dialog::FileDialog;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::mpsc,
};

#[derive(Serialize, Deserialize, Default)]
pub struct DeploymentApp {
    pub service_name: String,
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: String,
    pub config_path: Option<PathBuf>,
    pub jar_path: Option<PathBuf>,
    pub output_log: String,

    #[serde(skip)]
    pub is_deploying: bool,

    #[serde(skip)]
    pub config_file_dialog: FileDialog,

    #[serde(skip)]
    pub jar_file_dialog: FileDialog,

    #[serde(skip)]
    pub receiver: Option<mpsc::Receiver<String>>,
}

impl Clone for DeploymentApp {
    fn clone(&self) -> Self {
        Self {
            service_name: self.service_name.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            host: self.host.clone(),
            port: self.port.clone(),
            config_path: self.config_path.clone(),
            jar_path: self.jar_path.clone(),
            output_log: self.output_log.clone(),
            is_deploying: self.is_deploying,
            config_file_dialog: FileDialog::new(),
            jar_file_dialog: FileDialog::new(),
            receiver: None,
        }
    }
}

impl DeploymentApp {
    const APP_STATE_FILE: &'static str = "deploy-rs-state.json";

    pub fn new(_cc: &eframe::CreationContext) -> Self {
        DeploymentApp::load()
    }

    pub fn load() -> Self {
        if let Ok(json) = std::fs::read_to_string(Self::APP_STATE_FILE) {
            if let Ok(app_state) = serde_json::from_str(&json) {
                return app_state;
            }
        }
        DeploymentApp::default()
    }

    pub fn save_state(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::APP_STATE_FILE, json);
        }
    }

    pub fn handle_incoming_logs(&mut self, ctx: &Context) {
        if let Some(ref rx) = self.receiver {
            while let Ok(msg) = rx.try_recv() {
                if msg == "DEPLOYMENT_FINISHED" {
                    self.is_deploying = false;
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

        self.jar_file_dialog.update(ctx);
        if let Some(path) = self.jar_file_dialog.take_selected() {
            self.jar_path = Some(path);
        }
    }
}
