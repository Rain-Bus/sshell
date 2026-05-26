use crate::app::{App, Mode};
use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::view::{
    CredFormView, CredListView, DeleteConfirmView, FormView, HomeListView, ImportView,
    QuickSelectView, SearchView, SettingsView, ShellImportView, View,
};

static TERMINAL_RESTORED: AtomicBool = AtomicBool::new(false);

pub fn run() -> Result<()> {
    let _guard = TuiGuard::init()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::load()?;

    loop {
        terminal.draw(|frame| super::draw(frame, &mut app))?;
        if app.session.should_quit {
            break;
        }
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
        {
            handle_key(&mut app, key)?;
        }
    }

    Ok(())
}

struct TuiGuard;

impl TuiGuard {
    fn init() -> Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TuiGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

pub fn restore_terminal() -> Result<()> {
    if TERMINAL_RESTORED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let _ = disable_raw_mode();
    execute!(io::stdout(), Show, LeaveAlternateScreen)?;
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.session.should_quit = true;
        return Ok(());
    }
    match app.session.mode {
        Mode::Home => HomeListView.handle_key(app, key),
        Mode::Search => SearchView.handle_key(app, key),
        Mode::QuickSelect => QuickSelectView.handle_key(app, key),
        Mode::DeleteConfirm => DeleteConfirmView.handle_key(app, key),
        Mode::Form => FormView.handle_key(app, key),
        Mode::ImportSelector => ImportView.handle_key(app, key),
        Mode::ShellImport => ShellImportView.handle_key(app, key),
        Mode::Credentials => CredListView.handle_key(app, key),
        Mode::CredForm => CredFormView.handle_key(app, key),
        Mode::Settings => SettingsView.handle_key(app, key),
    }
}
