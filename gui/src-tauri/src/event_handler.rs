use crate::{app::ScenarioAppState, shared::SEPARATOR};
use scenario_rs::scenario::events::Event;
use std::sync::{mpsc, mpsc::Sender, Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub fn new_channel(app_handle: AppHandle) -> Sender<Event> {
    let (tx, rx) = mpsc::channel::<Event>();
    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn(async move {
        for event in rx {
            handle_event(&app_handle_clone, &event);

            if let Event::ScenarioCompleted | Event::ScenarioError(_) = event {
                break;
            }
        }
    });

    tx
}

fn handle_event(app_handle: &AppHandle, event: &Event) {
    match event {
        Event::ScenarioStarted => {
            log_message(app_handle, "Scenario started...\n");
            let _ = app_handle.emit("execution-status", true);
        }
        Event::StepStarted {
            index,
            total_steps,
            description,
        } => {
            let task_number = index + 1;
            log_message(
                app_handle,
                format!("{SEPARATOR}\n[{task_number}/{total_steps}] {description}\n"),
            );
        }
        Event::RemoteSudoBefore(command) => {
            log_message(app_handle, format!("Executing:\n{command}\n"));
        }
        Event::RemoteSudoChannelOutput(output) => {
            let output = output.trim();
            let truncated_output = output
                .chars()
                .take(1000)
                .collect::<String>()
                .trim()
                .to_string();
            log_message(app_handle, format!("{truncated_output}\n"));
            if output.len() > 1000 {
                log_message(app_handle, "...output truncated...\n");
            }
        }
        Event::SftpCopyBefore {
            source,
            destination,
        } => {
            log_message(
                app_handle,
                format!("Source:\n{source}\nDestination:\n{destination}\n"),
            );
        }
        Event::SftpCopyProgress { current, total } => {
            let percentage = (*current as f64 / *total as f64) * 100.0;
            log_message(app_handle, format!("Progress: {:.1}%\n", percentage));
        }
        Event::OnFailStepsStarted => {
            log_message(
                app_handle,
                format!("{SEPARATOR}\n[on_fail] Starting failure recovery steps\n"),
            );
        }
        Event::OnFailStepStarted {
            index,
            total_steps,
            description,
        } => {
            let task_number = index + 1;
            log_message(
                app_handle,
                format!("{SEPARATOR}\n[on_fail] [{task_number}/{total_steps}] {description}\n"),
            );
        }
        Event::ScenarioCompleted => {
            log_message(
                app_handle,
                format!("{SEPARATOR}\nScenario completed successfully!\n{SEPARATOR}\n"),
            );
            let _ = app_handle.emit("execution-status", false);
        }
        Event::ScenarioError(error) => {
            log_message(
                app_handle,
                format!("{SEPARATOR}\nScenario execution failed: {error}\n{SEPARATOR}\n"),
            );
            let _ = app_handle.emit("execution-status", false);
        }
        Event::StepCompleted => {
            log_message(app_handle, "Step completed\n");
        }
        Event::RemoteSudoAfter => {
            log_message(app_handle, "Remote sudo command completed\n");
        }
        Event::SftpCopyAfter => {
            log_message(app_handle, "SFTP copy finished\n");
        }
        Event::OnFailStepCompleted => {
            log_message(app_handle, "On-fail step completed\n");
        }
        Event::OnFailStepsCompleted => {
            log_message(
                app_handle,
                format!("{SEPARATOR}\nOn-fail steps completed\n"),
            );
        }
    }
}

fn log_message(app_handle: &AppHandle, message: impl AsRef<str>) {
    let state = app_handle.state::<Mutex<ScenarioAppState>>();
    let mut state = state.lock().unwrap();
    state.output_log.push_str(message.as_ref());
    let _ = app_handle.emit("log-update", ());
}
