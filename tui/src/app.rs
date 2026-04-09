use crate::file_browser::FileBrowser;
use scenario_rs::{
    scenario::{
        variables::required::VariableType,
        Scenario,
    },
    state::types::{
        ExecutionState, ExecutionStatus, OnFailStepExecState, StateDiff, StepExecState, StepStatus,
    },
    state::ExecutionStateManager,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
};

pub enum Screen {
    PickConfig,
    Variables,
    FilePicker,
    Executing,
    Done,
}

pub struct VariableField {
    pub name: String,
    pub label: String,
    pub value: String,
    pub read_only: bool,
    pub is_path: bool,
}

pub struct App {
    pub screen: Screen,
    pub scenario: Option<Scenario>,
    pub variable_fields: Vec<VariableField>,
    pub selected_field: usize,
    pub execution_state: ExecutionState,
    pub selected_step: usize,
    pub output_scroll: u16,
    pub diff_rx: Option<Receiver<StateDiff>>,
    pub should_quit: bool,
    pub file_browser: FileBrowser,
    pub config_error: Option<String>,
    pub debug_mode: bool,
}

impl App {
    pub fn with_scenario(scenario: Scenario) -> Self {
        let mut app = App::without_scenario();
        app.load_scenario(scenario);
        app
    }

    pub fn without_scenario() -> Self {
        App {
            screen: Screen::PickConfig,
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

    pub fn load_scenario(&mut self, scenario: Scenario) {
        let mut fields: Vec<VariableField> = scenario
            .variables()
            .required()
            .iter()
            .map(|(name, var)| VariableField {
                name: name.clone(),
                label: var.label().to_string(),
                value: var.value().to_string(),
                read_only: var.read_only(),
                is_path: matches!(var.var_type(), VariableType::Path),
            })
            .collect();

        fields.sort_by(|a, b| a.name.cmp(&b.name));

        let steps: Vec<StepExecState> = scenario
            .steps()
            .iter()
            .map(|step| {
                let on_fail_steps: Vec<OnFailStepExecState> = step
                    .on_fail_steps()
                    .iter()
                    .enumerate()
                    .map(|(i, ofs)| OnFailStepExecState {
                        index: i,
                        task_description: ofs.task().description().to_string(),
                        status: StepStatus::Pending,
                        progress: None,
                        output: String::new(),
                        errors: Vec::new(),
                    })
                    .collect();

                StepExecState {
                    index: step.index(),
                    task_description: step.task().description().to_string(),
                    status: StepStatus::Pending,
                    progress: None,
                    output: String::new(),
                    errors: Vec::new(),
                    on_fail_steps,
                }
            })
            .collect();

        self.execution_state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps,
        };
        self.variable_fields = fields;
        self.selected_field = 0;
        self.selected_step = 0;
        self.output_scroll = 0;
        self.diff_rx = None;
        self.scenario = Some(scenario);
        self.screen = Screen::Variables;
        self.config_error = None;
    }

    pub fn try_load_config(&mut self, path: PathBuf) {
        match Scenario::try_from(path) {
            Ok(scenario) => self.load_scenario(scenario),
            Err(e) => {
                self.config_error = Some(e.to_string());
            }
        }
    }

    pub fn open_file_picker_for_variable(&mut self) {
        if let Some(field) = self.variable_fields.get(self.selected_field) {
            if field.is_path && !field.read_only {
                self.file_browser = FileBrowser::from_cwd(None);
                self.screen = Screen::FilePicker;
            }
        }
    }

    pub fn confirm_file_pick(&mut self) {
        let selected_path = self
            .file_browser
            .selected_file()
            .map(|p| p.to_string_lossy().to_string());

        match &self.screen {
            Screen::PickConfig => {
                if let Some(path) = self.file_browser.selected_file() {
                    self.try_load_config(path.to_path_buf());
                }
            }
            Screen::FilePicker => {
                if let Some(path_str) = selected_path {
                    if let Some(field) = self.variable_fields.get_mut(self.selected_field) {
                        field.value = path_str;
                    }
                    self.screen = Screen::Variables;
                }
            }
            _ => {}
        }
    }

    pub fn cancel_file_pick(&mut self) {
        match &self.screen {
            Screen::PickConfig => self.should_quit = true,
            Screen::FilePicker => self.screen = Screen::Variables,
            _ => {}
        }
    }

    pub fn prepare_execution(&mut self) -> Option<(Scenario, ExecutionStateManager)> {
        let Some(scenario) = &mut self.scenario else {
            return None;
        };

        let variables: HashMap<String, String> = self
            .variable_fields
            .iter()
            .filter(|f| !f.read_only)
            .map(|f| (f.name.clone(), f.value.clone()))
            .collect();

        scenario.variables_mut().required_mut().upsert(variables);

        let (diff_tx, diff_rx) = mpsc::channel();
        let state_manager = ExecutionStateManager::new(self.execution_state.clone(), diff_tx);
        self.diff_rx = Some(diff_rx);
        self.screen = Screen::Executing;

        Some((scenario.clone(), state_manager))
    }

    #[cfg(not(tarpaulin_include))]
    pub fn start_execution(&mut self) {
        let debug_mode = self.debug_mode;
        if let Some((scenario, state_manager)) = self.prepare_execution() {
            thread::spawn(move || {
                scenario.execute(Some(&state_manager), debug_mode);
            });
        }
    }

    /// Drain pending state diffs from the channel and apply them.
    pub fn poll_diffs(&mut self) {
        let Some(rx) = &self.diff_rx else { return };

        while let Ok(diff) = rx.try_recv() {
            apply_diff(&mut self.execution_state, &diff);

            // Auto-select the currently running step
            match &diff {
                StateDiff::StepStatusChanged {
                    step_index,
                    status: StepStatus::Running,
                } => {
                    self.selected_step = *step_index;
                    self.output_scroll = 0;
                }
                _ => {}
            }

            // Transition to Done screen when execution finishes
            match &self.execution_state.status {
                ExecutionStatus::Completed | ExecutionStatus::Failed { .. } => {
                    self.screen = Screen::Done;
                }
                _ => {}
            }
        }
    }

    pub fn next_field(&mut self) {
        if !self.variable_fields.is_empty() {
            self.selected_field = (self.selected_field + 1) % self.variable_fields.len();
        }
    }

    pub fn prev_field(&mut self) {
        if !self.variable_fields.is_empty() {
            self.selected_field = self
                .selected_field
                .checked_sub(1)
                .unwrap_or(self.variable_fields.len() - 1);
        }
    }

    pub fn next_step(&mut self) {
        if !self.execution_state.steps.is_empty() {
            self.selected_step = (self.selected_step + 1) % self.execution_state.steps.len();
            self.output_scroll = 0;
        }
    }

    pub fn prev_step(&mut self) {
        if !self.execution_state.steps.is_empty() {
            self.selected_step = self
                .selected_step
                .checked_sub(1)
                .unwrap_or(self.execution_state.steps.len() - 1);
            self.output_scroll = 0;
        }
    }

    pub fn scroll_output_down(&mut self) {
        self.output_scroll = self.output_scroll.saturating_add(1);
    }

    pub fn scroll_output_up(&mut self) {
        self.output_scroll = self.output_scroll.saturating_sub(1);
    }

    pub fn type_char(&mut self, c: char) {
        if let Some(field) = self.variable_fields.get_mut(self.selected_field) {
            if !field.read_only {
                field.value.push(c);
            }
        }
    }

    pub fn backspace(&mut self) {
        if let Some(field) = self.variable_fields.get_mut(self.selected_field) {
            if !field.read_only {
                field.value.pop();
            }
        }
    }

    pub fn restart(&mut self) {
        if let Some(scenario) = self.scenario.take() {
            self.load_scenario(scenario);
        }
    }

    pub fn new_scenario(&mut self) {
        self.scenario = None;
        self.variable_fields.clear();
        self.selected_field = 0;
        self.execution_state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: Vec::new(),
        };
        self.selected_step = 0;
        self.output_scroll = 0;
        self.diff_rx = None;
        self.config_error = None;
        self.file_browser = FileBrowser::from_cwd(Some("toml".to_string()));
        self.screen = Screen::PickConfig;
    }

    pub fn toggle_debug_mode(&mut self) {
        self.debug_mode = !self.debug_mode;
    }
}

fn apply_diff(state: &mut ExecutionState, diff: &StateDiff) {
    match diff {
        StateDiff::ExecutionStatusChanged { status } => {
            state.status = status.clone();
        }
        StateDiff::StepStatusChanged { step_index, status } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                step.status = status.clone();
            }
        }
        StateDiff::StepProgressUpdated {
            step_index,
            progress,
        } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                step.progress = Some(progress.clone());
            }
        }
        StateDiff::StepOutputAppended { step_index, text } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                if !step.output.is_empty() {
                    step.output.push('\n');
                }
                step.output.push_str(text);
            }
        }
        StateDiff::StepErrorAdded { step_index, error } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                step.errors.push(error.clone());
            }
        }
        StateDiff::OnFailStepStatusChanged {
            step_index,
            on_fail_step_index,
            status,
        } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(*on_fail_step_index) {
                    ofs.status = status.clone();
                }
            }
        }
        StateDiff::OnFailStepProgressUpdated {
            step_index,
            on_fail_step_index,
            progress,
        } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(*on_fail_step_index) {
                    ofs.progress = Some(progress.clone());
                }
            }
        }
        StateDiff::OnFailStepOutputAppended {
            step_index,
            on_fail_step_index,
            text,
        } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(*on_fail_step_index) {
                    if !ofs.output.is_empty() {
                        ofs.output.push('\n');
                    }
                    ofs.output.push_str(text);
                }
            }
        }
        StateDiff::OnFailStepErrorAdded {
            step_index,
            on_fail_step_index,
            error,
        } => {
            if let Some(step) = state.steps.get_mut(*step_index) {
                if let Some(ofs) = step.on_fail_steps.get_mut(*on_fail_step_index) {
                    ofs.errors.push(error.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use scenario_rs::state::TaskProgress;

    use super::*;

    fn make_step(index: usize, desc: &str) -> StepExecState {
        StepExecState {
            index,
            task_description: desc.to_string(),
            status: StepStatus::Pending,
            progress: None,
            output: String::new(),
            errors: Vec::new(),
            on_fail_steps: vec![OnFailStepExecState {
                index: 0,
                task_description: "rollback".to_string(),
                status: StepStatus::Pending,
                progress: None,
                output: String::new(),
                errors: Vec::new(),
            }],
        }
    }

    fn make_app_with_steps(n: usize) -> App {
        let mut app = App::without_scenario();
        app.screen = Screen::Executing;
        app.execution_state.steps = (0..n)
            .map(|i| make_step(i, &format!("step {}", i)))
            .collect();
        app
    }

    fn make_app_with_fields() -> App {
        let mut app = App::without_scenario();
        app.screen = Screen::Variables;
        app.variable_fields = vec![
            VariableField {
                name: "path_var".into(),
                label: "Path Var".into(),
                value: String::new(),
                read_only: false,
                is_path: true,
            },
            VariableField {
                name: "str_var".into(),
                label: "String Var".into(),
                value: String::new(),
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
        app
    }

    #[test]
    fn without_scenario_starts_at_pick_config() {
        // Given & When
        let app = App::without_scenario();

        // Then
        assert!(matches!(app.screen, Screen::PickConfig));
        assert!(app.scenario.is_none());
        assert!(app.variable_fields.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn try_load_config_invalid_sets_error() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.try_load_config(PathBuf::from("nonexistent.toml"));

        // Then
        assert!(app.config_error.is_some());
        assert!(app.scenario.is_none());
    }

    #[test]
    fn try_load_config_valid_clears_error() {
        // Given
        let mut app = App::without_scenario();
        app.config_error = Some("old error".into());

        // When
        app.try_load_config(PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        ));

        // Then
        assert!(app.config_error.is_none());
        assert!(app.scenario.is_some());
        assert!(matches!(app.screen, Screen::Variables));
        assert!(!app.variable_fields.is_empty());
        assert!(!app.execution_state.steps.is_empty());
    }

    #[test]
    fn with_scenario_loads_correctly() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();

        // When
        let app = App::with_scenario(scenario);

        // Then
        assert!(matches!(app.screen, Screen::Variables));
        assert!(app.scenario.is_some());
    }

    #[test]
    fn load_scenario_detects_path_fields() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();

        // When
        let app = App::with_scenario(scenario);

        // Then
        let path_fields: Vec<_> = app.variable_fields.iter().filter(|f| f.is_path).collect();
        assert!(!path_fields.is_empty());
    }

    #[test]
    fn load_scenario_fields_sorted_by_name() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();

        // When
        let app = App::with_scenario(scenario);

        // Then
        let names: Vec<&str> = app.variable_fields.iter().map(|f| f.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn next_field_wraps() {
        // Given
        let mut app = make_app_with_fields();
        let len = app.variable_fields.len();

        // When
        for _ in 0..len {
            app.next_field();
        }

        // Then
        assert_eq!(app.selected_field, 0);
    }

    #[test]
    fn prev_field_wraps() {
        // Given
        let mut app = make_app_with_fields();

        // When
        app.prev_field();

        // Then
        assert_eq!(app.selected_field, app.variable_fields.len() - 1);
    }

    #[test]
    fn next_field_noop_when_empty() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.next_field();

        // Then
        assert_eq!(app.selected_field, 0);
    }

    #[test]
    fn prev_field_noop_when_empty() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.prev_field();

        // Then
        assert_eq!(app.selected_field, 0);
    }

    #[test]
    fn next_step_wraps() {
        // Given
        let mut app = make_app_with_steps(3);

        // When
        for _ in 0..3 {
            app.next_step();
        }

        // Then
        assert_eq!(app.selected_step, 0);
    }

    #[test]
    fn prev_step_wraps() {
        // Given
        let mut app = make_app_with_steps(3);

        // When
        app.prev_step();

        // Then
        assert_eq!(app.selected_step, 2);
    }

    #[test]
    fn next_step_noop_when_empty() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.next_step();

        // Then
        assert_eq!(app.selected_step, 0);
    }

    #[test]
    fn prev_step_noop_when_empty() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.prev_step();

        // Then
        assert_eq!(app.selected_step, 0);
    }

    #[test]
    fn step_navigation_resets_scroll() {
        // Given
        let mut app = make_app_with_steps(3);
        app.output_scroll = 10;

        // When
        app.next_step();

        // Then
        assert_eq!(app.output_scroll, 0);

        // Given
        app.output_scroll = 10;

        // When
        app.prev_step();

        // Then
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn scroll_output_down_and_up() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.scroll_output_down();
        app.scroll_output_down();

        // Then
        assert_eq!(app.output_scroll, 2);

        // When
        app.scroll_output_up();

        // Then
        assert_eq!(app.output_scroll, 1);
    }

    #[test]
    fn scroll_output_up_saturates_at_zero() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.scroll_output_up();

        // Then
        assert_eq!(app.output_scroll, 0);
    }

    #[test]
    fn type_char_appends_to_field() {
        // Given
        let mut app = make_app_with_fields();

        // When
        app.type_char('a');
        app.type_char('b');

        // Then
        assert_eq!(app.variable_fields[0].value, "ab");
    }

    #[test]
    fn type_char_noop_on_read_only() {
        // Given
        let mut app = make_app_with_fields();
        app.selected_field = 2;

        // When
        app.type_char('x');

        // Then
        assert_eq!(app.variable_fields[2].value, "fixed");
    }

    #[test]
    fn type_char_noop_when_no_fields() {
        // Given & When & Then
        let mut app = App::without_scenario();
        app.type_char('x');
    }

    #[test]
    fn backspace_removes_char() {
        // Given
        let mut app = make_app_with_fields();
        app.variable_fields[0].value = "abc".into();

        // When
        app.backspace();

        // Then
        assert_eq!(app.variable_fields[0].value, "ab");
    }

    #[test]
    fn backspace_noop_on_read_only() {
        // Given
        let mut app = make_app_with_fields();
        app.selected_field = 2;

        // When
        app.backspace();

        // Then
        assert_eq!(app.variable_fields[2].value, "fixed");
    }

    #[test]
    fn backspace_noop_when_no_fields() {
        // Given & When & Then
        let mut app = App::without_scenario();
        app.backspace();
    }

    #[test]
    fn open_file_picker_for_path_variable() {
        // Given
        let mut app = make_app_with_fields();
        app.selected_field = 0;

        // When
        app.open_file_picker_for_variable();

        // Then
        assert!(matches!(app.screen, Screen::FilePicker));
    }

    #[test]
    fn open_file_picker_noop_for_string_variable() {
        // Given
        let mut app = make_app_with_fields();
        app.selected_field = 1;

        // When
        app.open_file_picker_for_variable();

        // Then
        assert!(matches!(app.screen, Screen::Variables));
    }

    #[test]
    fn open_file_picker_noop_for_read_only() {
        // Given
        let mut app = make_app_with_fields();
        app.variable_fields[0].read_only = true;

        // When
        app.open_file_picker_for_variable();

        // Then
        assert!(matches!(app.screen, Screen::Variables));
    }

    #[test]
    fn open_file_picker_noop_when_no_fields() {
        // Given
        let mut app = App::without_scenario();
        app.screen = Screen::Variables;

        // When
        app.open_file_picker_for_variable();

        // Then
        assert!(matches!(app.screen, Screen::Variables));
    }

    #[test]
    fn cancel_file_pick_from_pick_config_quits() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.cancel_file_pick();

        // Then
        assert!(app.should_quit);
    }

    #[test]
    fn cancel_file_pick_from_file_picker_returns_to_variables() {
        // Given
        let mut app = make_app_with_fields();
        app.screen = Screen::FilePicker;

        // When
        app.cancel_file_pick();

        // Then
        assert!(matches!(app.screen, Screen::Variables));
    }

    #[test]
    fn cancel_file_pick_noop_on_other_screens() {
        // Given
        let mut app = make_app_with_steps(1);
        app.screen = Screen::Executing;

        // When
        app.cancel_file_pick();

        // Then
        assert!(!app.should_quit);
        assert!(matches!(app.screen, Screen::Executing));
    }

    #[test]
    fn confirm_file_pick_on_pick_config_loads_scenario() {
        // Given
        let mut app = App::without_scenario();
        let config_dir = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs"),
        );
        app.file_browser = FileBrowser::new(config_dir, Some("toml".to_string()));
        while app
            .file_browser
            .selected_entry()
            .map(|e| e.name.as_str() != "example-scenario.toml")
            .unwrap_or(false)
        {
            app.file_browser.select_next();
        }
        assert!(app.file_browser.selected_file().is_some());

        // When
        app.confirm_file_pick();

        // Then
        assert!(matches!(app.screen, Screen::Variables));
        assert!(app.scenario.is_some());
    }

    #[test]
    fn confirm_file_pick_on_file_picker_sets_value() {
        // Given
        let mut app = make_app_with_fields();
        app.screen = Screen::FilePicker;
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "").unwrap();
        app.file_browser = FileBrowser::new(tmp.path().to_path_buf(), None);
        while app
            .file_browser
            .selected_entry()
            .map(|e| e.is_dir)
            .unwrap_or(false)
        {
            app.file_browser.select_next();
        }

        // When
        app.confirm_file_pick();

        // Then
        assert!(matches!(app.screen, Screen::Variables));
        assert!(app.variable_fields[0].value.contains("test.txt"));
    }

    #[test]
    fn confirm_file_pick_noop_on_other_screens() {
        // Given & When & Then
        let mut app = make_app_with_steps(1);
        app.screen = Screen::Executing;
        app.confirm_file_pick();
    }

    #[test]
    fn prepare_execution_without_scenario_returns_none() {
        // Given
        let mut app = App::without_scenario();
        app.screen = Screen::Variables;

        // When
        let result = app.prepare_execution();

        // Then
        assert!(result.is_none());
        assert!(matches!(app.screen, Screen::Variables));
        assert!(app.diff_rx.is_none());
    }

    #[test]
    fn prepare_execution_with_scenario_returns_some() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();
        let mut app = App::with_scenario(scenario);
        app.variable_fields.iter_mut().for_each(|f| {
            if !f.read_only {
                f.value = "test_value".into();
            }
        });

        // When
        let result = app.prepare_execution();

        // Then
        assert!(result.is_some());
        assert!(matches!(app.screen, Screen::Executing));
        assert!(app.diff_rx.is_some());
    }

    #[test]
    fn prepare_execution_collects_editable_variables() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();
        let mut app = App::with_scenario(scenario);
        let editable_count = app.variable_fields.iter().filter(|f| !f.read_only).count();
        app.variable_fields.iter_mut().for_each(|f| {
            if !f.read_only {
                f.value = format!("val_{}", f.name);
            }
        });

        // When
        let (scenario, _sm) = app.prepare_execution().unwrap();

        // Then
        let required = scenario.variables().required();
        for field in app.variable_fields.iter().filter(|f| !f.read_only) {
            assert_eq!(
                required.get(&field.name).unwrap().value(),
                &format!("val_{}", field.name)
            );
        }
        assert!(editable_count > 0);
    }

    #[test]
    fn poll_diffs_without_receiver_is_noop() {
        // Given & When & Then
        let mut app = App::without_scenario();
        app.poll_diffs();
    }

    #[test]
    fn poll_diffs_applies_diffs() {
        // Given
        let mut app = make_app_with_steps(2);
        let (tx, rx) = mpsc::channel();
        app.diff_rx = Some(rx);
        tx.send(StateDiff::StepStatusChanged {
            step_index: 1,
            status: StepStatus::Running,
        })
        .unwrap();
        tx.send(StateDiff::StepOutputAppended {
            step_index: 1,
            text: "hello".into(),
        })
        .unwrap();
        drop(tx);

        // When
        app.poll_diffs();

        // Then
        assert_eq!(app.selected_step, 1);
        assert_eq!(app.output_scroll, 0);
        assert_eq!(app.execution_state.steps[1].status, StepStatus::Running);
        assert_eq!(app.execution_state.steps[1].output, "hello");
    }

    #[test]
    fn poll_diffs_transitions_to_done_on_completed() {
        // Given
        let mut app = make_app_with_steps(1);
        let (tx, rx) = mpsc::channel();
        app.diff_rx = Some(rx);
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Completed,
        })
        .unwrap();
        drop(tx);

        // When
        app.poll_diffs();

        // Then
        assert!(matches!(app.screen, Screen::Done));
    }

    #[test]
    fn poll_diffs_transitions_to_done_on_failed() {
        // Given
        let mut app = make_app_with_steps(1);
        let (tx, rx) = mpsc::channel();
        app.diff_rx = Some(rx);
        tx.send(StateDiff::ExecutionStatusChanged {
            status: ExecutionStatus::Failed {
                error: "oops".into(),
            },
        })
        .unwrap();
        drop(tx);

        // When
        app.poll_diffs();

        // Then
        assert!(matches!(app.screen, Screen::Done));
    }

    #[test]
    fn poll_diffs_non_running_status_does_not_auto_select() {
        // Given
        let mut app = make_app_with_steps(2);
        let (tx, rx) = mpsc::channel();
        app.diff_rx = Some(rx);
        tx.send(StateDiff::StepStatusChanged {
            step_index: 1,
            status: StepStatus::Completed,
        })
        .unwrap();
        drop(tx);

        // When
        app.poll_diffs();

        // Then
        assert_eq!(app.selected_step, 0);
    }

    #[test]
    fn apply_diff_execution_status_changed() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Idle,
            steps: vec![],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::ExecutionStatusChanged {
                status: ExecutionStatus::Running,
            },
        );

        // Then
        assert_eq!(state.status, ExecutionStatus::Running);
    }

    #[test]
    fn apply_diff_step_status_changed() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::StepStatusChanged {
                step_index: 0,
                status: StepStatus::Running,
            },
        );

        // Then
        assert_eq!(state.steps[0].status, StepStatus::Running);
    }

    #[test]
    fn apply_diff_step_status_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::StepStatusChanged {
                step_index: 99,
                status: StepStatus::Running,
            },
        );
    }

    #[test]
    fn apply_diff_step_progress_updated() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::StepProgressUpdated {
                step_index: 0,
                progress: TaskProgress::RemoteSudo {
                    command: "ls".into(),
                    output: String::new(),
                },
            },
        );

        // Then
        assert!(state.steps[0].progress.is_some());
    }

    #[test]
    fn apply_diff_step_progress_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::StepProgressUpdated {
                step_index: 99,
                progress: TaskProgress::RemoteSudo {
                    command: "ls".into(),
                    output: String::new(),
                },
            },
        );
    }

    #[test]
    fn apply_diff_step_output_appended() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::StepOutputAppended {
                step_index: 0,
                text: "line1".into(),
            },
        );
        apply_diff(
            &mut state,
            &StateDiff::StepOutputAppended {
                step_index: 0,
                text: "line2".into(),
            },
        );

        // Then
        assert_eq!(state.steps[0].output, "line1\nline2");
    }

    #[test]
    fn apply_diff_step_output_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::StepOutputAppended {
                step_index: 99,
                text: "x".into(),
            },
        );
    }

    #[test]
    fn apply_diff_step_error_added() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::StepErrorAdded {
                step_index: 0,
                error: "boom".into(),
            },
        );

        // Then
        assert_eq!(state.steps[0].errors, vec!["boom"]);
    }

    #[test]
    fn apply_diff_step_error_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::StepErrorAdded {
                step_index: 99,
                error: "x".into(),
            },
        );
    }

    #[test]
    fn apply_diff_on_fail_status_changed() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepStatusChanged {
                step_index: 0,
                on_fail_step_index: 0,
                status: StepStatus::Running,
            },
        );

        // Then
        assert_eq!(
            state.steps[0].on_fail_steps[0].status,
            StepStatus::Running
        );
    }

    #[test]
    fn apply_diff_on_fail_status_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepStatusChanged {
                step_index: 0,
                on_fail_step_index: 99,
                status: StepStatus::Running,
            },
        );
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepStatusChanged {
                step_index: 99,
                on_fail_step_index: 0,
                status: StepStatus::Running,
            },
        );
    }

    #[test]
    fn apply_diff_on_fail_progress_updated() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepProgressUpdated {
                step_index: 0,
                on_fail_step_index: 0,
                progress: TaskProgress::RemoteSudo {
                    command: "rollback".into(),
                    output: String::new(),
                },
            },
        );

        // Then
        assert!(state.steps[0].on_fail_steps[0].progress.is_some());
    }

    #[test]
    fn apply_diff_on_fail_progress_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepProgressUpdated {
                step_index: 0,
                on_fail_step_index: 99,
                progress: TaskProgress::RemoteSudo {
                    command: "x".into(),
                    output: String::new(),
                },
            },
        );
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepProgressUpdated {
                step_index: 99,
                on_fail_step_index: 0,
                progress: TaskProgress::RemoteSudo {
                    command: "x".into(),
                    output: String::new(),
                },
            },
        );
    }

    #[test]
    fn apply_diff_on_fail_output_appended() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepOutputAppended {
                step_index: 0,
                on_fail_step_index: 0,
                text: "a".into(),
            },
        );
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepOutputAppended {
                step_index: 0,
                on_fail_step_index: 0,
                text: "b".into(),
            },
        );

        // Then
        assert_eq!(state.steps[0].on_fail_steps[0].output, "a\nb");
    }

    #[test]
    fn apply_diff_on_fail_output_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepOutputAppended {
                step_index: 0,
                on_fail_step_index: 99,
                text: "x".into(),
            },
        );
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepOutputAppended {
                step_index: 99,
                on_fail_step_index: 0,
                text: "x".into(),
            },
        );
    }

    #[test]
    fn apply_diff_on_fail_error_added() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepErrorAdded {
                step_index: 0,
                on_fail_step_index: 0,
                error: "oops".into(),
            },
        );

        // Then
        assert_eq!(state.steps[0].on_fail_steps[0].errors, vec!["oops"]);
    }

    #[test]
    fn apply_diff_on_fail_error_out_of_bounds() {
        // Given
        let mut state = ExecutionState {
            status: ExecutionStatus::Running,
            steps: vec![make_step(0, "s")],
        };

        // When & Then
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepErrorAdded {
                step_index: 0,
                on_fail_step_index: 99,
                error: "x".into(),
            },
        );
        apply_diff(
            &mut state,
            &StateDiff::OnFailStepErrorAdded {
                step_index: 99,
                on_fail_step_index: 0,
                error: "x".into(),
            },
        );
    }

    #[test]
    fn restart_reloads_same_scenario() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();
        let mut app = App::with_scenario(scenario);
        app.screen = Screen::Done;
        app.execution_state.status = ExecutionStatus::Completed;
        app.selected_step = 2;
        app.output_scroll = 5;

        // When
        app.restart();

        // Then
        assert!(matches!(app.screen, Screen::Variables));
        assert!(app.scenario.is_some());
        assert_eq!(app.selected_step, 0);
        assert_eq!(app.execution_state.status, ExecutionStatus::Idle);
        assert!(app.execution_state.steps.iter().all(|s| s.status == StepStatus::Pending));
    }

    #[test]
    fn restart_without_scenario_is_noop() {
        // Given
        let mut app = App::without_scenario();
        app.screen = Screen::Done;

        // When
        app.restart();

        // Then
        assert!(matches!(app.screen, Screen::Done));
    }

    #[test]
    fn new_scenario_resets_to_file_picker() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();
        let mut app = App::with_scenario(scenario);
        app.screen = Screen::Done;

        // When
        app.new_scenario();

        // Then
        assert!(matches!(app.screen, Screen::PickConfig));
        assert!(app.scenario.is_none());
        assert!(app.variable_fields.is_empty());
        assert!(app.execution_state.steps.is_empty());
        assert_eq!(app.selected_field, 0);
        assert_eq!(app.selected_step, 0);
        assert!(app.config_error.is_none());
    }

    #[test]
    fn toggle_debug_mode_flips_flag() {
        // Given
        let mut app = App::without_scenario();

        // When
        app.toggle_debug_mode();

        // Then
        assert!(app.debug_mode);

        // When
        app.toggle_debug_mode();

        // Then
        assert!(!app.debug_mode);
    }

    #[test]
    fn debug_mode_preserved_across_restart() {
        // Given
        let path = PathBuf::from(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../example_configs/example-scenario.toml"),
        );
        let scenario = Scenario::try_from(path).unwrap();
        let mut app = App::with_scenario(scenario);
        app.debug_mode = true;

        // When
        app.restart();

        // Then
        assert!(app.debug_mode);
    }
}
