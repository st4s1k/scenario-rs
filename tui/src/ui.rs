use crate::app::{App, Screen};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use scenario_rs::state::types::{ExecutionStatus, StepStatus, TaskProgress};

pub fn draw(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::PickConfig => draw_file_browser(frame, app, " Select Config File (*.toml) "),
        Screen::Variables => draw_variables_screen(frame, app),
        Screen::FilePicker => draw_file_browser(frame, app, " Select File "),
        Screen::Executing => draw_execution_screen(frame, app),
        Screen::Done => draw_done_screen(frame, app),
    }
}

fn draw_file_browser(frame: &mut Frame, app: &App, title: &str) {
    let area = frame.area();
    let browser = &app.file_browser;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    // Current directory header
    let dir_display = browser.current_dir.to_string_lossy();
    let mut header_spans = vec![
        Span::styled(" ", Style::default()),
        Span::styled(dir_display.as_ref(), Style::default().fg(Color::Cyan).bold()),
    ];
    if let Some(err) = &browser.error {
        header_spans.push(Span::styled(
            format!("  {}", err),
            Style::default().fg(Color::Red),
        ));
    }
    if let Some(err) = &app.config_error {
        header_spans.push(Span::styled(
            format!("  {}", err),
            Style::default().fg(Color::Red),
        ));
    }
    let header = Paragraph::new(Line::from(header_spans))
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(header, chunks[0]);

    // File list
    let items: Vec<ListItem> = browser
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = i == browser.selected;
            let (icon, color) = if entry.is_dir {
                ("/ ", Color::Blue)
            } else {
                ("  ", Color::White)
            };
            let style = if is_selected {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(color)
            };
            let prefix = if is_selected { "> " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::raw(icon),
                Span::styled(&entry.name, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL));
    frame.render_widget(list, chunks[1]);

    // Help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑/↓", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Navigate  "),
        Span::styled("Enter/→", Style::default().fg(Color::Green).bold()),
        Span::raw(" Open  "),
        Span::styled("Backspace/←", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Parent  "),
        Span::styled("Esc", Style::default().fg(Color::Red).bold()),
        Span::raw(" Cancel"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}

fn draw_variables_screen(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if app.variable_fields.is_empty() {
        let title = if app.debug_mode {
            " scenario-rs [DEBUG] "
        } else {
            " scenario-rs "
        };
        let paragraph = Paragraph::new("No required variables. Press Enter to execute.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title),
            );
        frame.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Variable form
    let items: Vec<ListItem> = app
        .variable_fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let is_selected = i == app.selected_field;
            let cursor_indicator = if is_selected { "> " } else { "  " };
            let read_only_tag = if field.read_only {
                " [read-only]"
            } else if field.is_path {
                " [path]"
            } else {
                ""
            };

            let label_style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let value_style = if field.read_only {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };

            let line = Line::from(vec![
                Span::styled(cursor_indicator, label_style),
                Span::styled(&field.label, label_style),
                Span::styled(read_only_tag, Style::default().fg(Color::DarkGray)),
                Span::raw(": "),
                Span::styled(&field.value, value_style),
                if is_selected && !field.read_only {
                    Span::styled("_", Style::default().fg(Color::Yellow))
                } else {
                    Span::raw("")
                },
            ]);

            ListItem::new(line)
        })
        .collect();

    let title = if app.debug_mode {
        " Required Variables [DEBUG] "
    } else {
        " Required Variables "
    };
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title),
    );
    frame.render_widget(list, chunks[0]);

    // Help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Next  "),
        Span::styled("Shift+Tab", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Prev  "),
        Span::styled("Ctrl+B", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Browse  "),
        Span::styled("Ctrl+D", Style::default().fg(Color::Yellow).bold()),
        Span::raw(" Debug  "),
        Span::styled("Enter", Style::default().fg(Color::Green).bold()),
        Span::raw(" Execute  "),
        Span::styled("Esc", Style::default().fg(Color::Red).bold()),
        Span::raw(" Quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[1]);
}

fn draw_execution_screen(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status header
            Constraint::Min(0),   // Steps list + detail
            Constraint::Length(3), // Help bar
        ])
        .split(area);

    // Status header
    let state = &app.execution_state;
    let (status_text, status_color) = match &state.status {
        ExecutionStatus::Idle => ("Idle", Color::DarkGray),
        ExecutionStatus::Running => ("Running", Color::Yellow),
        ExecutionStatus::Completed => ("Completed", Color::Green),
        ExecutionStatus::Failed { .. } => ("Failed", Color::Red),
    };

    let total = state.steps.len();
    let completed = state
        .steps
        .iter()
        .filter(|s| s.status == StepStatus::Completed)
        .count();

    let mut header_spans = vec![
        Span::styled(" Status: ", Style::default().bold()),
        Span::styled(status_text, Style::default().fg(status_color).bold()),
        Span::raw(format!("  Steps: {}/{}", completed, total)),
    ];
    if app.debug_mode {
        header_spans.push(Span::styled("  [DEBUG]", Style::default().fg(Color::Yellow).bold()));
    }
    let header = Paragraph::new(Line::from(header_spans))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Execution "),
    );
    frame.render_widget(header, chunks[0]);

    // Steps + detail split
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    draw_steps_list(frame, app, content_chunks[0]);
    draw_step_detail(frame, app, content_chunks[1]);

    // Help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑/↓", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Navigate  "),
        Span::styled("Esc/q", Style::default().fg(Color::Red).bold()),
        Span::raw(" Quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}

fn draw_steps_list(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.execution_state;

    let items: Vec<ListItem> = state
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let is_selected = i == app.selected_step;
            let (icon, icon_color) = match step.status {
                StepStatus::Pending => (".", Color::DarkGray),
                StepStatus::Running => ("*", Color::Yellow),
                StepStatus::Completed => ("+", Color::Green),
                StepStatus::Failed => ("x", Color::Red),
                StepStatus::Skipped => ("-", Color::DarkGray),
            };

            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default().fg(icon_color)),
                Span::styled(
                    format!("[{}/{}] ", i + 1, state.steps.len()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(&step.task_description, style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Steps "),
    );
    frame.render_widget(list, area);
}

fn draw_step_detail(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.execution_state;
    let Some(step) = state.steps.get(app.selected_step) else {
        let empty = Paragraph::new("No steps").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detail "),
        );
        frame.render_widget(empty, area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Progress / status
            Constraint::Min(0),   // Output
            Constraint::Length(4), // Errors (if any)
        ])
        .split(area);

    // Progress bar or status
    match &step.progress {
        Some(TaskProgress::SftpCopy {
            bytes_transferred,
            bytes_total,
            elapsed_ms,
            ..
        }) if *bytes_total > 0 => {
            let ratio = *bytes_transferred as f64 / *bytes_total as f64;
            let elapsed_secs = *elapsed_ms as f64 / 1000.0;
            let speed = if elapsed_secs > 0.1 {
                let mbps = (*bytes_transferred as f64 / (1024.0 * 1024.0)) / elapsed_secs;
                format!(" -- {:.1}s, {:.2} MB/s", elapsed_secs, mbps)
            } else {
                String::new()
            };
            let label = format!(
                "{} / {}{}",
                format_bytes(*bytes_transferred),
                format_bytes(*bytes_total),
                speed
            );
            let gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Transfer Progress "),
                )
                .gauge_style(Style::default().fg(Color::Cyan))
                .ratio(ratio.min(1.0))
                .label(label);
            frame.render_widget(gauge, chunks[0]);
        }
        Some(TaskProgress::RemoteSudo { command, .. }) => {
            let cmd_line = Paragraph::new(Line::from(vec![
                Span::styled("Command: ", Style::default().bold()),
                Span::styled(command, Style::default().fg(Color::Cyan)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Task "),
            );
            frame.render_widget(cmd_line, chunks[0]);
        }
        _ => {
            let (status_text, color) = match step.status {
                StepStatus::Pending => ("Pending", Color::DarkGray),
                StepStatus::Running => ("Running...", Color::Yellow),
                StepStatus::Completed => ("Completed", Color::Green),
                StepStatus::Failed => ("Failed", Color::Red),
                StepStatus::Skipped => ("Skipped", Color::DarkGray),
            };
            let status_para = Paragraph::new(Span::styled(
                format!(" {}", status_text),
                Style::default().fg(color).bold(),
            ))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Status "),
            );
            frame.render_widget(status_para, chunks[0]);
        }
    }

    // Output view
    let output_text = if step.output.is_empty() {
        "(no output yet)".to_string()
    } else {
        step.output.clone()
    };
    let output = Paragraph::new(output_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Output "),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.output_scroll, 0));
    frame.render_widget(output, chunks[1]);

    // Errors
    if !step.errors.is_empty() {
        let error_text: String = step.errors.join("\n");
        let errors = Paragraph::new(error_text)
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Errors ")
                    .border_style(Style::default().fg(Color::Red)),
            )
            .wrap(Wrap { trim: false });
        frame.render_widget(errors, chunks[2]);
    } else {
        let no_errors = Paragraph::new("").block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Errors "),
        );
        frame.render_widget(no_errors, chunks[2]);
    }
}

fn draw_done_screen(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let state = &app.execution_state;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Result banner
            Constraint::Min(0),   // Steps list + detail
            Constraint::Length(3), // Help
        ])
        .split(area);

    // Result banner
    let (title, color, message) = match &state.status {
        ExecutionStatus::Completed => (
            " Scenario Completed ",
            Color::Green,
            "All steps executed successfully.".to_string(),
        ),
        ExecutionStatus::Failed { error } => (
            " Scenario Failed ",
            Color::Red,
            error.clone(),
        ),
        _ => (
            " Scenario ",
            Color::White,
            "Execution finished.".to_string(),
        ),
    };

    let banner = Paragraph::new(vec![
        Line::raw(""),
        Line::from(Span::styled(
            &message,
            Style::default().fg(color).bold(),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(color)),
    )
    .wrap(Wrap { trim: false });
    frame.render_widget(banner, chunks[0]);

    // Steps + detail split (same as execution screen)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    draw_steps_list(frame, app, content_chunks[0]);
    draw_step_detail(frame, app, content_chunks[1]);

    // Help bar
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" ↑/↓", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Navigate  "),
        Span::styled("PgUp/PgDn", Style::default().fg(Color::Cyan).bold()),
        Span::raw(" Scroll  "),
        Span::styled("r", Style::default().fg(Color::Green).bold()),
        Span::raw(" Re-run  "),
        Span::styled("n", Style::default().fg(Color::Green).bold()),
        Span::raw(" New Scenario  "),
        Span::styled("Esc/q", Style::default().fg(Color::Red).bold()),
        Span::raw(" Quit"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, VariableField};
    use crate::file_browser::FileBrowser;
    use ratatui::{backend::TestBackend, Terminal};
    use scenario_rs::state::types::{
        ExecutionState, OnFailStepExecState, StepExecState, TaskProgress,
    };

    fn render(app: &App) {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| draw(frame, app)).unwrap();
    }

    fn make_step(index: usize, desc: &str, status: StepStatus) -> StepExecState {
        StepExecState {
            index,
            task_description: desc.to_string(),
            status,
            progress: None,
            output: String::new(),
            errors: Vec::new(),
            on_fail_steps: Vec::new(),
        }
    }

    fn app_at_screen(screen: Screen) -> App {
        App {
            screen,
            scenario: None,
            variable_fields: Vec::new(),
            selected_field: 0,
            execution_state: ExecutionState {
                status: ExecutionStatus::Idle,
                steps: Vec::new(),
            },
            selected_step: 0,
            output_scroll: 0,
            diff_rx: None,
            should_quit: false,
            file_browser: FileBrowser::from_cwd(Some("toml".to_string())),
            config_error: None,
            debug_mode: false,
        }
    }

    #[test]
    fn format_bytes_b() {
        // Given & When & Then
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(100), "100 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn format_bytes_kb() {
        // Given & When & Then
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn format_bytes_mb() {
        // Given & When & Then
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 5), "5.0 MB");
    }

    #[test]
    fn format_bytes_gb() {
        // Given & When & Then
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 2), "2.0 GB");
    }

    #[test]
    fn draw_pick_config_no_panic() {
        // Given & When & Then
        let app = app_at_screen(Screen::PickConfig);
        render(&app);
    }

    #[test]
    fn draw_pick_config_with_errors() {
        // Given
        let mut app = app_at_screen(Screen::PickConfig);
        app.file_browser.error = Some("dir error".into());
        app.config_error = Some("config error".into());

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_file_picker_no_panic() {
        // Given & When & Then
        let app = app_at_screen(Screen::FilePicker);
        render(&app);
    }

    #[test]
    fn draw_variables_empty_no_panic() {
        // Given & When & Then
        let app = app_at_screen(Screen::Variables);
        render(&app);
    }

    #[test]
    fn draw_variables_with_fields() {
        // Given
        let mut app = app_at_screen(Screen::Variables);
        app.variable_fields = vec![
            VariableField {
                name: "path_var".into(),
                label: "Path Var".into(),
                value: "/some/path".into(),
                read_only: false,
                is_path: true,
            },
            VariableField {
                name: "str_var".into(),
                label: "String Var".into(),
                value: "hello".into(),
                read_only: false,
                is_path: false,
            },
            VariableField {
                name: "ro_var".into(),
                label: "Read Only".into(),
                value: "fixed".into(),
                read_only: true,
                is_path: false,
            },
        ];
        app.selected_field = 1;

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_idle_no_steps() {
        // Given & When & Then
        let app = app_at_screen(Screen::Executing);
        render(&app);
    }

    #[test]
    fn draw_executing_running_with_steps() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        app.execution_state.steps = vec![
            make_step(0, "step 1", StepStatus::Completed),
            make_step(1, "step 2", StepStatus::Running),
            make_step(2, "step 3", StepStatus::Pending),
        ];
        app.selected_step = 1;

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_with_sftp_progress() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        let mut step = make_step(0, "copy file", StepStatus::Running);
        step.progress = Some(TaskProgress::SftpCopy {
            source: "local.txt".into(),
            destination: "/remote/path".into(),
            bytes_transferred: 512 * 1024,
            bytes_total: 1024 * 1024,
            elapsed_ms: 1000,
        });
        app.execution_state.steps = vec![step];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_with_sftp_zero_total() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        let mut step = make_step(0, "copy file", StepStatus::Running);
        step.progress = Some(TaskProgress::SftpCopy {
            source: "local.txt".into(),
            destination: "/remote/path".into(),
            bytes_transferred: 0,
            bytes_total: 0,
            elapsed_ms: 0,
        });
        app.execution_state.steps = vec![step];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_with_remote_sudo_progress() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        let mut step = make_step(0, "run cmd", StepStatus::Running);
        step.progress = Some(TaskProgress::RemoteSudo {
            command: "ls -la".into(),
            output: "file1\nfile2".into(),
        });
        app.execution_state.steps = vec![step];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_with_output_and_errors() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        let mut step = make_step(0, "step 1", StepStatus::Failed);
        step.output = "some output\nline2".into();
        step.errors = vec!["error 1".into(), "error 2".into()];
        app.execution_state.steps = vec![step];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_with_scroll() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        let mut step = make_step(0, "step 1", StepStatus::Running);
        step.output = (0..50).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        app.execution_state.steps = vec![step];
        app.output_scroll = 10;

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_all_step_statuses() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        app.execution_state.steps = vec![
            make_step(0, "pending", StepStatus::Pending),
            make_step(1, "running", StepStatus::Running),
            make_step(2, "completed", StepStatus::Completed),
            make_step(3, "failed", StepStatus::Failed),
            make_step(4, "skipped", StepStatus::Skipped),
        ];

        // When & Then
        for i in 0..5 {
            app.selected_step = i;
            render(&app);
        }
    }

    #[test]
    fn draw_executing_failed_status() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Failed {
            error: "something broke".into(),
        };
        app.execution_state.steps = vec![make_step(0, "s", StepStatus::Failed)];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_done_completed() {
        // Given
        let mut app = app_at_screen(Screen::Done);
        app.execution_state.status = ExecutionStatus::Completed;
        app.execution_state.steps = vec![
            make_step(0, "s1", StepStatus::Completed),
            make_step(1, "s2", StepStatus::Completed),
        ];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_done_failed() {
        // Given
        let mut app = app_at_screen(Screen::Done);
        app.execution_state.status = ExecutionStatus::Failed {
            error: "deployment failed".into(),
        };
        let mut step = make_step(0, "s1", StepStatus::Failed);
        step.errors = vec!["error msg".into()];
        app.execution_state.steps = vec![step];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_done_other_status() {
        // Given
        let mut app = app_at_screen(Screen::Done);
        app.execution_state.status = ExecutionStatus::Idle;

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_done_with_step_detail() {
        // Given
        let mut app = app_at_screen(Screen::Done);
        app.execution_state.status = ExecutionStatus::Completed;
        let mut step = make_step(0, "deploy", StepStatus::Completed);
        step.output = "deployed successfully".into();
        step.on_fail_steps = vec![OnFailStepExecState {
            index: 0,
            task_description: "rollback".into(),
            status: StepStatus::Pending,
            progress: None,
            output: String::new(),
            errors: Vec::new(),
        }];
        app.execution_state.steps = vec![step];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_variables_empty_debug_mode() {
        // Given
        let mut app = app_at_screen(Screen::Variables);
        app.debug_mode = true;

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_variables_with_fields_debug_mode() {
        // Given
        let mut app = app_at_screen(Screen::Variables);
        app.debug_mode = true;
        app.variable_fields = vec![VariableField {
            name: "var".into(),
            label: "Var".into(),
            value: "val".into(),
            read_only: false,
            is_path: false,
        }];

        // When & Then
        render(&app);
    }

    #[test]
    fn draw_executing_with_debug() {
        // Given
        let mut app = app_at_screen(Screen::Executing);
        app.execution_state.status = ExecutionStatus::Running;
        app.execution_state.steps = vec![make_step(0, "step", StepStatus::Running)];
        app.debug_mode = true;

        // When & Then
        render(&app);
    }
}
