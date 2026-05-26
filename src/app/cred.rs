use super::{TextEditing, char_len};
use crate::app::AuthKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredFormField {
    Name,
    Kind,
    Value,
}

impl CredFormField {
    pub const ALL: [CredFormField; 3] = [
        CredFormField::Name,
        CredFormField::Kind,
        CredFormField::Value,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Kind => "Kind",
            Self::Value => "Value",
        }
    }

    pub fn is_toggle(self) -> bool {
        matches!(self, Self::Kind)
    }

    pub fn is_text(self) -> bool {
        matches!(self, Self::Name | Self::Value)
    }
}

#[derive(Debug, Clone)]
pub struct CredFormState {
    pub edit_name: Option<String>,
    pub active: CredFormField,
    pub cursor: usize,
    pub name: String,
    pub kind: AuthKind,
    pub value: String,
}

impl CredFormState {
    pub fn blank() -> Self {
        Self {
            edit_name: None,
            active: CredFormField::Name,
            cursor: 0,
            name: String::new(),
            kind: AuthKind::Password,
            value: String::new(),
        }
    }

    pub fn next_field(&mut self) {
        let idx = CredFormField::ALL
            .iter()
            .position(|&f| f == self.active)
            .unwrap_or(0);
        self.active = CredFormField::ALL[(idx + 1) % CredFormField::ALL.len()];
        self.cursor = char_len(self.active_text());
    }

    pub fn prev_field(&mut self) {
        let idx = CredFormField::ALL
            .iter()
            .position(|&f| f == self.active)
            .unwrap_or(0);
        self.active =
            CredFormField::ALL[(idx + CredFormField::ALL.len() - 1) % CredFormField::ALL.len()];
        self.cursor = char_len(self.active_text());
    }

    pub fn field_value(&self, field: CredFormField) -> &str {
        match field {
            CredFormField::Name => &self.name,
            CredFormField::Value => &self.value,
            _ => "",
        }
    }
}

impl TextEditing for CredFormState {
    fn active_text(&self) -> &str {
        match self.active {
            CredFormField::Name => &self.name,
            CredFormField::Value => &self.value,
            _ => "",
        }
    }

    fn active_text_mut(&mut self) -> Option<&mut String> {
        match self.active {
            CredFormField::Name => Some(&mut self.name),
            CredFormField::Value => Some(&mut self.value),
            _ => None,
        }
    }

    fn cursor(&self) -> usize {
        self.cursor
    }

    fn set_cursor(&mut self, pos: usize) {
        self.cursor = pos;
    }
}
