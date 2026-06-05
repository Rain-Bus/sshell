use crate::app::{App, Mode};
use crate::config::{ConnectionType, SyncBackend};
use anyhow::Result;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::view::{
    ActionMenuView, CredFormView, CredListView, DeleteConfirmView, FormView, HomeListView,
    ImportView, QuickSelectView, SearchView, SettingsView, View,
};

static TERMINAL_RESTORED: AtomicBool = AtomicBool::new(false);

pub fn run() -> Result<()> {
    let _guard = TuiGuard::init()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::load()?;

    if app.config.settings.sync_on_start {
        let result = match app.config.settings.backend {
            SyncBackend::Gist => crate::sync::gist::sync(&mut app.config),
            SyncBackend::Webdav => crate::sync::webdav::sync(&mut app.config),
        };
        if let Err(err) = result {
            app.toast(err.to_string(), false);
        }
    }

    spawn_latency_probes(&app);

    loop {
        terminal.draw(|frame| super::draw(frame, &mut app))?;
        if app.session.should_quit {
            break;
        }
        spawn_latency_probes(&app);
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
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
        Mode::ActionMenu => ActionMenuView.handle_key(app, key),
        Mode::Search => SearchView.handle_key(app, key),
        Mode::QuickSelect => QuickSelectView.handle_key(app, key),
        Mode::DeleteConfirm => DeleteConfirmView.handle_key(app, key),
        Mode::Form => FormView.handle_key(app, key),
        Mode::ImportSelector => ImportView.handle_key(app, key),
        Mode::Credentials => CredListView.handle_key(app, key),
        Mode::CredForm => CredFormView.handle_key(app, key),
        Mode::Settings => SettingsView.handle_key(app, key),
    }
}

fn spawn_latency_probes(app: &App) {
    use crate::app::latency::{CacheEntry, STALE_SECS};
    use std::time::Instant;

    let stale_duration = Duration::from_secs(STALE_SECS);
    let now = Instant::now();

    // Collect keys that need probing (missing or stale)
    let cache = app.session.latency.lock().unwrap();
    let mut to_probe: Vec<String> = Vec::new();
    for (_, profile) in app.entries() {
        if let ConnectionType::Ssh { host, port, .. } = &profile.kind {
            let key = format!("{host}:{port}");
            let needs_probe = match cache.get(&key) {
                None => true,
                Some(entry) => now.duration_since(entry.checked_at) >= stale_duration,
            };
            if needs_probe && !to_probe.contains(&key) {
                to_probe.push(key);
            }
        }
    }
    drop(cache);

    for key in to_probe {
        // Re-check under lock to avoid duplicate spawns
        {
            let cache = app.session.latency.lock().unwrap();
            if let Some(entry) = cache.get(&key)
                && now.duration_since(entry.checked_at) < stale_duration
            {
                continue;
            }
        }
        // Mark as "in-flight" by inserting a fresh entry
        {
            let mut cache = app.session.latency.lock().unwrap();
            cache.insert(key.clone(), CacheEntry {
                status: crate::app::latency::LatencyStatus::Unknown,
                checked_at: now,
            });
        }
        let cache_clone = app.session.latency.clone();
        let host_port = key.clone();
        std::thread::spawn(move || {
            let parts: Vec<&str> = host_port.splitn(2, ':').collect();
            let (host, port) = match parts.as_slice() {
                [h, p] => (*h, p.parse::<u16>().unwrap_or(22)),
                _ => return,
            };
            let status = crate::app::latency::probe(host, port);
            if let Ok(mut cache) = cache_clone.lock() {
                cache.insert(host_port, CacheEntry {
                    status,
                    checked_at: Instant::now(),
                });
            }
        });
    }
}
