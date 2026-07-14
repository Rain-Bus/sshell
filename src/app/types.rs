use crate::import::ImportCandidate;

use super::cred::CredFormState;
use super::form::FormState;
use super::latency::LatencyCache;
use super::settings::SettingsState;

// ── Text editing trait ──────────────────────────────────────

pub trait TextEditing {
    fn active_text(&self) -> &str;
    fn active_text_mut(&mut self) -> Option<&mut String>;
    fn cursor(&self) -> usize;
    fn set_cursor(&mut self, pos: usize);

    fn insert_char(&mut self, c: char) {
        let pos = self.cursor().min(char_len(self.active_text()));
        let byte_pos = char_to_byte_index(self.active_text(), pos);
        if let Some(field) = self.active_text_mut() {
            field.insert(byte_pos, c);
            self.set_cursor(pos + 1);
        }
    }

    fn delete_char(&mut self) {
        let pos = self.cursor().min(char_len(self.active_text()));
        if pos == 0 {
            return;
        }
        let start = char_to_byte_index(self.active_text(), pos - 1);
        let end = char_to_byte_index(self.active_text(), pos);
        if let Some(field) = self.active_text_mut() {
            field.replace_range(start..end, "");
        }
        self.set_cursor(pos - 1);
    }

    fn delete_next_char(&mut self) {
        let pos = self.cursor().min(char_len(self.active_text()));
        if pos >= char_len(self.active_text()) {
            return;
        }
        let start = char_to_byte_index(self.active_text(), pos);
        let end = char_to_byte_index(self.active_text(), pos + 1);
        if let Some(field) = self.active_text_mut() {
            field.replace_range(start..end, "");
        }
        self.set_cursor(pos);
    }

    fn move_cursor_left(&mut self) {
        let pos = self.cursor().min(char_len(self.active_text()));
        if pos > 0 {
            self.set_cursor(pos - 1);
        }
    }

    fn move_cursor_right(&mut self) {
        let pos = self.cursor().min(char_len(self.active_text()));
        let len = char_len(self.active_text());
        self.set_cursor((pos + 1).min(len));
    }

    fn cursor_home(&mut self) {
        self.set_cursor(0);
    }

    fn cursor_end(&mut self) {
        self.set_cursor(char_len(self.active_text()));
    }

    fn clear_field(&mut self) {
        if let Some(field) = self.active_text_mut() {
            field.clear();
        }
        self.set_cursor(0);
    }
}

// ── Form navigation trait ────────────────────────────────────

pub trait FormNav: TextEditing {
    fn nav_next(&mut self);
    fn nav_prev(&mut self);
    fn active_is_toggle(&self) -> bool;
    fn active_is_text(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormAction {
    Toggle,
    Save,
    Cancel,
}

pub fn char_len(value: &str) -> usize {
    value.chars().count()
}

fn char_to_byte_index(value: &str, char_pos: usize) -> usize {
    value
        .char_indices()
        .nth(char_pos)
        .map(|(idx, _)| idx)
        .unwrap_or(value.len())
}

// ── Types ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Home,
    ActionMenu,
    Search,
    QuickSelect,
    Form,
    DeleteConfirm,
    ImportSelector,
    Credentials,
    CredForm,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickSortMode {
    Smart,
    Usage,
    Added,
    Name,
}

impl QuickSortMode {
    pub fn next(self) -> Self {
        match self {
            Self::Smart => Self::Usage,
            Self::Usage => Self::Added,
            Self::Added => Self::Name,
            Self::Name => Self::Smart,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Smart => "smart",
            Self::Usage => "usage",
            Self::Added => "added",
            Self::Name => "name",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    Password,
    PrivateKey,
}

// ── Toast ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub success: bool,
    pub born: std::time::Instant,
}

// ── Action Menu ────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ActionMenuSession {
    pub cursor: usize,
}

// ── Session ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Session {
    pub mode: Mode,
    pub home: HomeSession,
    pub action_menu: ActionMenuSession,
    pub form: FormState,
    pub credentials: CredentialSession,
    pub import: ImportSession,
    pub toast: Option<Toast>,
    pub should_quit: bool,
    pub settings: SettingsState,
    pub latency: LatencyCache,
}

#[derive(Debug, Clone)]
pub struct HomeSession {
    pub selected: usize,
    pub search: String,
    pub quick_sort: QuickSortMode,
}

#[derive(Debug, Clone)]
pub struct CredentialSession {
    pub selected: usize,
    pub form: CredFormState,
}

#[derive(Debug, Clone, Default)]
pub struct ImportSession {
    pub cursor: usize,
    pub candidates: Vec<ImportCandidate>,
    pub selected: Vec<bool>,
    pub shell_candidates: Vec<crate::config::ShellCandidate>,
    pub shell_selected: Vec<bool>,
}

impl Session {
    pub fn new() -> Self {
        Self {
            mode: Mode::Home,
            home: HomeSession::default(),
            action_menu: ActionMenuSession::default(),
            form: FormState::blank(),
            credentials: CredentialSession::default(),
            import: ImportSession::default(),
            toast: None,
            should_quit: false,
            settings: SettingsState::default(),
            latency: LatencyCache::default(),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for HomeSession {
    fn default() -> Self {
        Self {
            selected: 0,
            search: String::new(),
            quick_sort: QuickSortMode::Smart,
        }
    }
}

impl Default for CredentialSession {
    fn default() -> Self {
        Self {
            selected: 0,
            form: CredFormState::blank(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct EditingState {
        value: String,
        cursor: usize,
    }

    impl TextEditing for EditingState {
        fn active_text(&self) -> &str {
            &self.value
        }

        fn active_text_mut(&mut self) -> Option<&mut String> {
            Some(&mut self.value)
        }

        fn cursor(&self) -> usize {
            self.cursor
        }

        fn set_cursor(&mut self, pos: usize) {
            self.cursor = pos;
        }
    }

    #[test]
    fn text_editing_inserts_at_character_cursor() {
        let mut state = EditingState {
            value: "你a".to_string(),
            cursor: 1,
        };

        state.insert_char('好');

        assert_eq!(state.value, "你好a");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn text_editing_backspace_removes_previous_character() {
        let mut state = EditingState {
            value: "你好a".to_string(),
            cursor: 2,
        };

        state.delete_char();

        assert_eq!(state.value, "你a");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn text_editing_delete_removes_next_character() {
        let mut state = EditingState {
            value: "你a好".to_string(),
            cursor: 1,
        };

        state.delete_next_char();

        assert_eq!(state.value, "你好");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn text_editing_cursor_moves_by_character() {
        let mut state = EditingState {
            value: "你a好".to_string(),
            cursor: 0,
        };

        state.move_cursor_right();
        state.move_cursor_right();
        assert_eq!(state.cursor, 2);

        state.move_cursor_left();
        assert_eq!(state.cursor, 1);

        state.cursor_end();
        assert_eq!(state.cursor, 3);
    }
}
