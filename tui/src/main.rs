use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use scenario_rs::scenario::Scenario;
use std::{io, path::PathBuf, process, time::Duration};

mod app;
mod file_browser;
mod ui;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    /// Path to the TOML config file (opens file picker if omitted)
    #[arg(short, long, value_name = "TOML_FILE")]
    config_path: Option<PathBuf>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let app = match cli.config_path {
        Some(path) => {
            let scenario: Scenario = match Scenario::try_from(path) {
                Ok(s) => s,
                Err(error) => {
                    eprintln!("Scenario initialization failed: {}", error);
                    process::exit(1);
                }
            };
            App::with_scenario(scenario)
        }
        None => App::without_scenario(),
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app;
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        // Poll state diffs from the execution thread
        app.poll_diffs();

        terminal.draw(|frame| ui::draw(frame, app))?;

        // Use a short poll timeout so we can refresh for state updates
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(app, key.code, key.modifiers);
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    match &app.screen {
        app::Screen::PickConfig | app::Screen::FilePicker => match code {
            KeyCode::Esc => app.cancel_file_pick(),
            KeyCode::Up | KeyCode::Char('k') => app.file_browser.select_prev(),
            KeyCode::Down | KeyCode::Char('j') => app.file_browser.select_next(),
            KeyCode::Left | KeyCode::Backspace => app.file_browser.go_up(),
            KeyCode::Right | KeyCode::Enter => {
                if app.file_browser.selected_file().is_some() {
                    app.confirm_file_pick();
                } else {
                    app.file_browser.enter();
                }
            }
            _ => {}
        },
        app::Screen::Variables => match code {
            KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.should_quit = true
            }
            KeyCode::Char('b') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.open_file_picker_for_variable()
            }
            KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                app.toggle_dry_run()
            }
            KeyCode::Tab | KeyCode::Down => app.next_field(),
            KeyCode::BackTab | KeyCode::Up => app.prev_field(),
            KeyCode::Enter => app.start_execution(),
            KeyCode::Backspace => app.backspace(),
            KeyCode::Char(c) => app.type_char(c),
            _ => {}
        },
        app::Screen::Executing => match code {
            KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => app.next_step(),
            KeyCode::Up | KeyCode::Char('k') => app.prev_step(),
            KeyCode::PageDown => app.scroll_output_down(),
            KeyCode::PageUp => app.scroll_output_up(),
            _ => {}
        },
        app::Screen::Done => match code {
            KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('r') => app.restart(),
            KeyCode::Char('n') => app.new_scenario(),
            KeyCode::Down | KeyCode::Char('j') => app.next_step(),
            KeyCode::Up | KeyCode::Char('k') => app.prev_step(),
            KeyCode::PageDown => app.scroll_output_down(),
            KeyCode::PageUp => app.scroll_output_up(),
            _ => {}
        },
    }
}
