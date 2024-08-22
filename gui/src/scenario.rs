use crate::{
    app::ScenarioApp,
    lifecycle::{execution_lifecycle, LifecycleHandler},
    shared::SEPARATOR,
};
use colored::Colorize;
use egui_file_dialog::FileDialog;
use scenario_rs::{
    config::ScenarioConfig,
    scenario::{
        credentials::Credentials,
        server::Server,
        variables::required::RequiredVariables,
        Scenario,
    },
};
use std::{sync::mpsc, thread};

pub fn start_scenario(app: &mut ScenarioApp) {
    if !app.is_executing {
        app.is_executing = true;
        let (tx, rx) = mpsc::channel();

        let app_state = ScenarioApp {
            service_name: app.service_name.clone(),
            username: app.username.clone(),
            password: app.password.clone(),
            host: app.host.clone(),
            port: app.port.clone(),
            config_path: app.config_path.clone(),
            jar_path: app.jar_path.clone(),
            output_log: app.output_log.clone(),
            is_executing: false,
            config_file_dialog: FileDialog::new(),
            jar_file_dialog: FileDialog::new(),
            receiver: None,
        };

        thread::spawn(move || {
            run_scenario(app_state, tx.clone());
            let _ = tx.send("SCENARIO_FINISHED".to_string());
        });

        app.receiver = Some(rx);
    }
}

fn run_scenario(app_state: ScenarioApp, tx: mpsc::Sender<String>) {
    LifecycleHandler::try_initialize(tx.clone());

    if let Err(e) = execute_scenario(app_state, &tx) {
        let mut log_message = String::new();
        log_message.push_str(&format!("{SEPARATOR}\n"));
        log_message.push_str(&format!("Scenario failed: {}\n", e));
        log_message.push_str(&format!("{SEPARATOR}\n"));
        let _ = tx.send(log_message);
    }
}

fn execute_scenario(app_state: ScenarioApp, tx: &mpsc::Sender<String>) -> Result<(), String> {
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

    let scenario = Scenario::new(server, credentials, config, required_variables)
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
