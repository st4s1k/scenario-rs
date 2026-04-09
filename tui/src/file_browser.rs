use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub struct FileBrowser {
    pub current_dir: PathBuf,
    pub entries: Vec<FsEntry>,
    pub selected: usize,
    pub extension_filter: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

impl FileBrowser {
    pub fn new(start_dir: PathBuf, extension_filter: Option<String>) -> Self {
        let mut browser = FileBrowser {
            current_dir: start_dir,
            entries: Vec::new(),
            selected: 0,
            extension_filter,
            error: None,
        };
        browser.refresh();
        browser
    }

    pub fn from_cwd(extension_filter: Option<String>) -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::new(cwd, extension_filter)
    }

    pub fn refresh(&mut self) {
        self.entries.clear();
        self.error = None;

        let read_dir = match fs::read_dir(&self.current_dir) {
            Ok(rd) => rd,
            Err(e) => {
                self.error = Some(format!("Cannot read directory: {}", e));
                return;
            }
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = path.is_dir();

            if is_dir {
                dirs.push(FsEntry {
                    name: format!("{}/", name),
                    path,
                    is_dir: true,
                });
            } else {
                let show = match &self.extension_filter {
                    Some(ext) => path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.eq_ignore_ascii_case(ext))
                        .unwrap_or(false),
                    None => true,
                };
                if show {
                    files.push(FsEntry {
                        name,
                        path,
                        is_dir: false,
                    });
                }
            }
        }

        dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

        self.entries = dirs;
        self.entries.extend(files);
        self.selected = 0;
    }

    pub fn enter(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                self.current_dir = entry.path.clone();
                self.refresh();
            }
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.refresh();
        }
    }

    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected = (self.selected + 1) % self.entries.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.entries.is_empty() {
            self.selected = self
                .selected
                .checked_sub(1)
                .unwrap_or(self.entries.len() - 1);
        }
    }

    pub fn selected_entry(&self) -> Option<&FsEntry> {
        self.entries.get(self.selected)
    }

    pub fn selected_file(&self) -> Option<&Path> {
        self.selected_entry()
            .filter(|e| !e.is_dir)
            .map(|e| e.path.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_dir() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().to_path_buf();

        fs::create_dir(base.join("subdir")).unwrap();
        fs::write(base.join("config.toml"), "").unwrap();
        fs::write(base.join("data.json"), "").unwrap();
        fs::write(base.join("readme.txt"), "").unwrap();

        (tmp, base)
    }

    #[test]
    fn new_lists_entries() {
        // Given
        let (_tmp, base) = create_test_dir();

        // When
        let browser = FileBrowser::new(base, None);

        // Then
        assert!(browser.error.is_none());
        assert_eq!(browser.selected, 0);
        assert_eq!(browser.entries.len(), 4);
    }

    #[test]
    fn extension_filter_shows_only_matching_files() {
        // Given
        let (_tmp, base) = create_test_dir();

        // When
        let browser = FileBrowser::new(base, Some("toml".to_string()));

        // Then
        let file_names: Vec<&str> = browser
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(file_names, vec!["config.toml"]);
    }

    #[test]
    fn extension_filter_is_case_insensitive() {
        // Given
        let (_tmp, base) = create_test_dir();
        fs::write(base.join("upper.TOML"), "").unwrap();

        // When
        let browser = FileBrowser::new(base, Some("toml".to_string()));

        // Then
        let file_names: Vec<&str> = browser
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(file_names, vec!["config.toml", "upper.TOML"]);
    }

    #[test]
    fn directories_listed_before_files() {
        // Given
        let (_tmp, base) = create_test_dir();

        // When
        let browser = FileBrowser::new(base, None);

        // Then
        assert!(browser.entries[0].is_dir);
        assert!(!browser.entries.last().unwrap().is_dir);
    }

    #[test]
    fn select_next_wraps_around() {
        // Given
        let (_tmp, base) = create_test_dir();
        let mut browser = FileBrowser::new(base, None);
        let len = browser.entries.len();

        // When
        for _ in 0..len {
            browser.select_next();
        }

        // Then
        assert_eq!(browser.selected, 0);
    }

    #[test]
    fn select_prev_wraps_around() {
        // Given
        let (_tmp, base) = create_test_dir();
        let mut browser = FileBrowser::new(base, None);

        // When
        browser.select_prev();

        // Then
        assert_eq!(browser.selected, browser.entries.len() - 1);
    }

    #[test]
    fn select_next_noop_on_empty() {
        // Given
        let tmp = tempfile::tempdir().unwrap();
        let mut browser = FileBrowser::new(tmp.path().to_path_buf(), Some("xyz".to_string()));

        // When
        browser.select_next();

        // Then
        assert_eq!(browser.selected, 0);
    }

    #[test]
    fn select_prev_noop_on_empty() {
        // Given
        let tmp = tempfile::tempdir().unwrap();
        let mut browser = FileBrowser::new(tmp.path().to_path_buf(), Some("xyz".to_string()));

        // When
        browser.select_prev();

        // Then
        assert_eq!(browser.selected, 0);
    }

    #[test]
    fn enter_navigates_into_directory() {
        // Given
        let (_tmp, base) = create_test_dir();
        let mut browser = FileBrowser::new(base.clone(), None);
        assert!(browser.entries[0].is_dir);

        // When
        browser.enter();

        // Then
        assert_eq!(browser.current_dir, base.join("subdir"));
    }

    #[test]
    fn enter_does_nothing_on_file() {
        // Given
        let (_tmp, base) = create_test_dir();
        let mut browser = FileBrowser::new(base.clone(), None);
        while browser.entries[browser.selected].is_dir {
            browser.select_next();
        }
        let dir_before = browser.current_dir.clone();

        // When
        browser.enter();

        // Then
        assert_eq!(browser.current_dir, dir_before);
    }

    #[test]
    fn enter_does_nothing_when_empty() {
        // Given
        let tmp = tempfile::tempdir().unwrap();
        let mut browser = FileBrowser::new(tmp.path().to_path_buf(), Some("xyz".to_string()));
        let dir_before = browser.current_dir.clone();

        // When
        browser.enter();

        // Then
        assert_eq!(browser.current_dir, dir_before);
    }

    #[test]
    fn go_up_navigates_to_parent() {
        // Given
        let (_tmp, base) = create_test_dir();
        let mut browser = FileBrowser::new(base.join("subdir"), None);

        // When
        browser.go_up();

        // Then
        assert_eq!(browser.current_dir, base);
    }

    #[test]
    fn selected_entry_returns_correct_entry() {
        // Given
        let (_tmp, base) = create_test_dir();
        let mut browser = FileBrowser::new(base, None);

        // When & Then
        let entry = browser.selected_entry().unwrap();
        assert!(entry.is_dir);

        browser.select_next();
        let entry = browser.selected_entry().unwrap();
        assert_eq!(entry.name, "config.toml");
    }

    #[test]
    fn selected_entry_returns_none_when_empty() {
        // Given & When
        let tmp = tempfile::tempdir().unwrap();
        let browser = FileBrowser::new(tmp.path().to_path_buf(), Some("xyz".to_string()));

        // Then
        assert!(browser.selected_entry().is_none());
    }

    #[test]
    fn selected_file_returns_none_for_directory() {
        // Given
        let (_tmp, base) = create_test_dir();
        let browser = FileBrowser::new(base, None);

        // When & Then
        assert!(browser.entries[0].is_dir);
        assert!(browser.selected_file().is_none());
    }

    #[test]
    fn selected_file_returns_path_for_file() {
        // Given
        let (_tmp, base) = create_test_dir();
        let mut browser = FileBrowser::new(base, None);
        while browser.entries[browser.selected].is_dir {
            browser.select_next();
        }

        // When & Then
        assert!(browser.selected_file().is_some());
    }

    #[test]
    fn refresh_on_invalid_dir_sets_error() {
        // Given
        let mut browser = FileBrowser {
            current_dir: PathBuf::from("/nonexistent_dir_12345"),
            entries: Vec::new(),
            selected: 0,
            extension_filter: None,
            error: None,
        };

        // When
        browser.refresh();

        // Then
        assert!(browser.error.is_some());
        assert!(browser.entries.is_empty());
    }

    #[test]
    fn from_cwd_creates_browser() {
        // Given & When
        let browser = FileBrowser::from_cwd(None);

        // Then
        assert_eq!(browser.current_dir, env::current_dir().unwrap());
        assert!(browser.error.is_none());
    }

    #[test]
    fn from_cwd_with_filter() {
        // Given & When
        let browser = FileBrowser::from_cwd(Some("rs".to_string()));

        // Then
        assert_eq!(browser.extension_filter, Some("rs".to_string()));
    }

    #[test]
    fn entries_sorted_case_insensitive() {
        // Given
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        fs::write(base.join("Zebra.txt"), "").unwrap();
        fs::write(base.join("apple.txt"), "").unwrap();
        fs::write(base.join("Banana.txt"), "").unwrap();

        // When
        let browser = FileBrowser::new(base.to_path_buf(), None);

        // Then
        let names: Vec<&str> = browser.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["apple.txt", "Banana.txt", "Zebra.txt"]);
    }
}
