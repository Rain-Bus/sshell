use crossterm::event::{KeyCode, KeyEvent};

pub enum ListAction {
    None,
    Cancel,
    Select,
}

pub fn handle_list_nav(cursor: &mut usize, len: usize, key: KeyEvent) -> ListAction {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') if len > 0 => {
            *cursor = (*cursor + 1) % len;
            ListAction::None
        }
        KeyCode::Up | KeyCode::Char('k') if len > 0 => {
            *cursor = (*cursor + len - 1) % len;
            ListAction::None
        }
        KeyCode::Esc => ListAction::Cancel,
        KeyCode::Enter => ListAction::Select,
        _ => ListAction::None,
    }
}
