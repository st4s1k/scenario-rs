use colored::Colorize;
use deploy_rs_core::data::{
    config::{
        RemoteSudoConfig,
        ScenarioConfig,
        SftpCopyConfig,
        StepConfig,
    },
    lifecycles::{
        ExecutionLifecycle,
        RemoteSudoLifecycle,
        RollbackLifecycle,
        RollbackStepLifecycle,
        SftpCopyLifecycle,
        StepLifecycle,
    },
    Credentials,
    RequiredVariables,
    Scenario,
    Server,
};
use eframe::egui;
use egui_file_dialog::FileDialog;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::PathBuf,
    sync::mpsc,
    sync::OnceLock,
    thread,
};

#[derive(Serialize, Deserialize, Default)]
struct DeploymentApp {
    service_name: String,
    username: String,
    password: String,
    host: String,
    port: String,
    config_path: Option<PathBuf>,
    jar_path: Option<PathBuf>,
    output_log: String,

    #[serde(skip)]
    is_deploying: bool,

    #[serde(skip)]
    config_file_dialog: FileDialog,

    #[serde(skip)]
    jar_file_dialog: FileDialog,

    #[serde(skip)]
    receiver: Option<mpsc::Receiver<String>>,
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
            config_file_dialog: FileDialog::new(), // Create a new FileDialog, can't clone the existing one
            jar_file_dialog: FileDialog::new(),    // Create a new FileDialog, can't clone the existing one
            receiver: None, // Receivers can't be cloned, so reset to None
        }
    }
}

impl DeploymentApp {
    fn new(_cc: &eframe::CreationContext) -> Self {
        DeploymentApp::load()
    }

    fn load() -> Self {
        if let Ok(json) = std::fs::read_to_string("deploy-rs-state.json") {
            if let Ok(app_state) = serde_json::from_str(&json) {
                return app_state;
            }
        }
        DeploymentApp::default()
    }

    fn save_state(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("deploy-rs-state.json", json);
        }
    }

    fn start_deployment(&mut self) {
        if !self.is_deploying {
            self.is_deploying = true;
            let (tx, rx) = mpsc::channel();

            // Passing a reference to self instead of cloning it
            let service_name = self.service_name.clone();
            let username = self.username.clone();
            let password = self.password.clone();
            let host = self.host.clone();
            let port = self.port.clone();
            let config_path = self.config_path.clone();
            let jar_path = self.jar_path.clone();
            let output_log = self.output_log.clone();

            // Start the deployment in a new thread
            thread::spawn(move || {
                let app_state = DeploymentApp {
                    service_name,
                    username,
                    password,
                    host,
                    port,
                    config_path,
                    jar_path,
                    output_log,
                    is_deploying: false, // Set this to false after the deployment finishes
                    config_file_dialog: FileDialog::new(),
                    jar_file_dialog: FileDialog::new(),
                    receiver: None,
                };
                run_deployment(app_state, tx.clone());
                let _ = tx.send("DEPLOYMENT_FINISHED".to_string()); // Signal end of deployment
            });

            self.receiver = Some(rx);
        }
    }
}

impl eframe::App for DeploymentApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        catppuccin_egui::set_theme(&ctx, catppuccin_egui::FRAPPE);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Deployment Tool");

            egui::Grid::new("deployment_tool_grid")
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Service Name:");
                    ui.text_edit_singleline(&mut self.service_name);
                    ui.end_row();

                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                    ui.end_row();

                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                    ui.end_row();

                    ui.label("Host:");
                    ui.text_edit_singleline(&mut self.host);
                    ui.end_row();

                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.port);
                    ui.end_row();

                    ui.label("Config Path:");
                    ui.group(|ui| {
                        let config_path_str = self.config_path.as_ref().map_or_else(
                            || String::from(""),
                            |p| p.file_name().map_or_else(|| String::from(""), |f| f.to_string_lossy().to_string()),
                        );
                        ui.text_edit_singleline(&mut config_path_str.clone());
                        if ui.button("Select Config File").clicked() {
                            self.config_file_dialog.select_file();
                        }
                    });
                    ui.end_row();

                    ui.label("JAR Path:");
                    ui.group(|ui| {
                        let jar_path_str = self.jar_path.as_ref().map_or_else(
                            || String::from(""),
                            |p| p.file_name().map_or_else(|| String::from(""), |f| f.to_string_lossy().to_string()),
                        );
                        ui.text_edit_singleline(&mut jar_path_str.clone());
                        if ui.button("Select JAR File").clicked() {
                            self.jar_file_dialog.select_file();
                        }
                    });
                    ui.end_row();

                    if ui.button("Deploy").clicked() {
                        self.start_deployment();
                    }
                });

            // Handle incoming log messages without blocking the UI
            if let Some(ref rx) = self.receiver {
                while let Ok(msg) = rx.try_recv() {
                    if msg == "DEPLOYMENT_FINISHED" {
                        self.is_deploying = false; // Reset deployment flag when the signal is received
                    } else {
                        self.output_log.push_str(&msg);
                    }
                }

                ctx.request_repaint_after(std::time::Duration::from_millis(100)); // Request a repaint to update the UI periodically
            }

            ui.separator();
            ui.label("Deployment Log:");
            ui.add(
                egui::TextEdit::multiline(&mut self.output_log)
                    .code_editor()
                    .font(egui::TextStyle::Monospace)
                    .desired_width(f32::INFINITY)
                    .desired_rows(10),
            );

            // Update the file dialogs
            self.config_file_dialog.update(ctx);
            if let Some(path) = self.config_file_dialog.take_selected() {
                self.config_path = Some(path);
            }

            self.jar_file_dialog.update(ctx);
            if let Some(path) = self.jar_file_dialog.take_selected() {
                self.jar_path = Some(path);
            }
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_state();
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Deployment Tool",
        options,
        Box::new(|cc| Ok(Box::new(DeploymentApp::new(cc)))),
    )
}

const SEPARATOR: &str = "------------------------------------------------------------";

fn run_deployment(app_state: DeploymentApp, tx: mpsc::Sender<String>) {
    LoggerLifecycle::try_initialize(tx.clone());

    if let Err(e) = deploy(app_state, &tx) {
        let mut log_message = String::new();
        log_message.push_str(&format!("{SEPARATOR}\n"));
        log_message.push_str(&format!("Deployment failed: {}\n", e));
        log_message.push_str(&format!("{SEPARATOR}\n"));
        let _ = tx.send(log_message);
    }
}

fn deploy(app_state: DeploymentApp, tx: &mpsc::Sender<String>) -> Result<(), String> {
    let mut log_message = String::new();

    log_message.push_str(&format!("{SEPARATOR}\n"));
    let server = Server::new(&app_state.host, &app_state.port);
    let credentials = Credentials::new(app_state.username.clone(), app_state.password.clone());

    let config = ScenarioConfig::try_from(app_state.config_path.clone().ok_or_else(|| "No config file selected".to_string())?)
        .map_err(|e| e.to_string())?;

    let timestamp = chrono::Local::now().format("%Y-%m-%dT%H%M%S%:z").to_string();
    let local_jar_path = app_state.jar_path.ok_or_else(|| "No JAR file selected".to_string())?;
    let local_jar_basename = local_jar_path.file_name().ok_or_else(|| "Invalid JAR file path".to_string())?
        .to_string_lossy().to_string();

    let required_variables = RequiredVariables::new([
        ("service_name".to_string(), app_state.service_name.clone()),
        ("username".to_string(), app_state.username.clone()),
        ("timestamp".to_string(), timestamp.clone()),
        ("local_jar_path".to_string(), local_jar_path.to_string_lossy().to_string()),
        ("local_jar_basename".to_string(), local_jar_basename.clone()),
    ]);

    let deploy_scenario = Scenario::new(server, credentials, config, required_variables)
        .map_err(|e| e.to_string())?;

    let lifecycle = execution_lifecycle();
    deploy_scenario.execute_with_lifecycle(lifecycle)
        .map_err(|e| e.to_string())?;

    log_message.push_str(&format!("{}\n", SEPARATOR));
    log_message.push_str(&format!("{}\n", "Deployment completed successfully!".cyan()));
    log_message.push_str(&format!("{}\n", SEPARATOR));
    let _ = tx.send(log_message);

    Ok(())
}

#[derive(Debug)]
struct LoggerLifecycle {
    tx: mpsc::Sender<String>,
}

impl LoggerLifecycle {
    fn new(tx: mpsc::Sender<String>) -> Self {
        Self { tx }
    }

    fn log_remote_sudo_before(&self, remote_sudo: &RemoteSudoConfig) {
        let log_message = format!(
            "{}\n{}\n",
            "Executing:".yellow(),
            remote_sudo.command().bold()
        );
        let _ = self.tx.send(log_message);
    }

    fn log_remote_sudo_channel_established(&self, channel: &mut dyn Read) {
        let mut output = String::new();
        if channel.read_to_string(&mut output).is_err() {
            let log_message = format!("{}\nChannel output is not a valid UTF-8\n{}\n", SEPARATOR, SEPARATOR);
            let _ = self.tx.send(log_message);
            return;
        }
        let output = output.trim();
        let truncated_output = output.chars().take(1000).collect::<String>().trim().to_string();
        let log_message = format!("{}\n", truncated_output);
        let _ = self.tx.send(log_message);
        if output.len() > 1000 {
            let log_message = "...output truncated...\n".to_string();
            let _ = self.tx.send(log_message);
        }
    }

    fn log_sftp_copy_before(&self, sftp_copy: &SftpCopyConfig) {
        let log_message = format!(
            "{}\n{}\n{}\n{}\n",
            "Source:".yellow(),
            sftp_copy.source_path().bold(),
            "Destination:".yellow(),
            sftp_copy.destination_path().bold()
        );
        let _ = self.tx.send(log_message);
    }

    fn log_rollback_before(&self, step: &StepConfig) {
        if step.rollback_steps().is_none() {
            let log_message = format!("{}\n[{}] No rollback actions found\n", SEPARATOR, "rollback".red());
            let _ = self.tx.send(log_message);
        }
    }

    fn log_rollback_step_before(&self, index: usize, rollback_step: &StepConfig, rollback_steps: &Vec<StepConfig>) {
        let step_number = index + 1;
        let total_rollback_steps = rollback_steps.len();
        let description = rollback_step.description();
        let log_message = format!("{}\n[{}] [{}/{}] {}\n", SEPARATOR, "rollback".red(), step_number, total_rollback_steps, description.purple());
        let _ = self.tx.send(log_message);
    }
}

static LOGGER: OnceLock<LoggerLifecycle> = OnceLock::new();

impl LoggerLifecycle {
    fn try_initialize(tx: mpsc::Sender<String>) {
        LOGGER.get_or_init(|| LoggerLifecycle::new(tx));
    }
}

fn execution_lifecycle() -> ExecutionLifecycle {
    let mut lifecycle = ExecutionLifecycle::default();
    lifecycle.step = step_lifecycle();
    lifecycle
}

fn step_lifecycle() -> StepLifecycle {
    let mut lifecycle = StepLifecycle::default();
    lifecycle.before = log_step_before;
    lifecycle.remote_sudo = remote_sudo_lifecycle();
    lifecycle.sftp_copy = sftp_copy_lifecycle();
    lifecycle.rollback = rollback_lifecycle();
    lifecycle
}

fn remote_sudo_lifecycle() -> RemoteSudoLifecycle {
    let mut lifecycle = RemoteSudoLifecycle::default();
    lifecycle.before = log_remote_sudo_before;
    lifecycle.channel_established = log_remote_sudo_channel_established;
    lifecycle
}

fn sftp_copy_lifecycle() -> SftpCopyLifecycle {
    let mut lifecycle = SftpCopyLifecycle::default();
    lifecycle.before = log_sftp_copy_before;
    lifecycle
}

fn rollback_lifecycle() -> RollbackLifecycle {
    let mut lifecycle = RollbackLifecycle::default();
    lifecycle.before = log_rollback_before;
    lifecycle.step = rollback_step_lifecycle();
    lifecycle
}

fn rollback_step_lifecycle() -> RollbackStepLifecycle {
    let mut lifecycle = RollbackStepLifecycle::default();
    lifecycle.before = log_rollback_step_before;
    lifecycle
}

fn log_step_before(index: usize, step: &StepConfig, steps: &Vec<StepConfig>) {
    if let Some(logger) = LOGGER.get() {
        let step_number: usize = index + 1;
        let description = step.description();
        let total_steps: usize = steps.len();
        let log_message = format!(
            "{}\n{}\n",
            SEPARATOR,
            format!("[{}/{}] {}", step_number, total_steps, description).purple()
        );
        let _ = logger.tx.send(log_message);
    }
}

fn log_remote_sudo_before(remote_sudo: &RemoteSudoConfig) {
    if let Some(logger) = LOGGER.get() {
        logger.log_remote_sudo_before(remote_sudo);
    }
}

fn log_remote_sudo_channel_established(channel: &mut dyn Read) {
    if let Some(logger) = LOGGER.get() {
        logger.log_remote_sudo_channel_established(channel);
    }
}

fn log_sftp_copy_before(sftp_copy: &SftpCopyConfig) {
    if let Some(logger) = LOGGER.get() {
        logger.log_sftp_copy_before(sftp_copy);
    }
}

fn log_rollback_before(step: &StepConfig) {
    if let Some(logger) = LOGGER.get() {
        logger.log_rollback_before(step);
    }
}

fn log_rollback_step_before(index: usize, rollback_step: &StepConfig, rollback_steps: &Vec<StepConfig>) {
    if let Some(logger) = LOGGER.get() {
        logger.log_rollback_step_before(index, rollback_step, rollback_steps);
    }
}
