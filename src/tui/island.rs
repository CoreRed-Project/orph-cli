use crate::cli::OutputFlags;
use crate::ipc;
use crate::models::config::ConfigEntry;
use crate::models::diagnostics::{HealthSnapshot, NetSnapshot, SysSnapshot};
use crate::models::pet::Pet;
use crate::services::diagnostics;
use crate::services::health;
use crate::services::pet_events;
use crate::services::pet_service;
use crate::services::telemetry::CommandCount;
use crate::tui::{art, log_format, theme};
use anyhow::{Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::*;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph, Wrap};
use std::collections::VecDeque;
use std::io;
use std::io::BufRead;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const DAEMON_TICK: Duration = Duration::from_millis(400);
const FALLBACK_TICK: Duration = Duration::from_millis(1500);
const LOG_MAX_LINES: usize = 400;
const LOG_TAIL: usize = 80;
const SPARKLEN: usize = 48;
const CPU_ALERT: f64 = 90.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataSource {
    Daemon,
    LocalFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivePane {
    Metrics,
    Operations,
    Companion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Command,
    RenamePet,
    ScriptLauncher,
}

struct AppState {
    data_source: DataSource,
    sys: SysSnapshot,
    health: HealthSnapshot,
    net: NetSnapshot,
    configs: Vec<ConfigEntry>,
    db_size_kb: u64,
    loadavg: Option<f64>,
    cpu_history: VecDeque<u64>,
    pet: Option<Pet>,
    telemetry_top: Vec<CommandCount>,
    logs: VecDeque<String>,
    log_scroll: usize,
    status_line: String,
    last_error: Option<String>,
    input_mode: InputMode,
    active_pane: ActivePane,
    prompt: String,
    show_stats: bool,
    scripts: Vec<String>,
    script_selected: usize,
    frame_count: u64, // For subtle animations
    storage_stats: Option<crate::services::db::StorageStats>,
    running_script: Option<(String, Instant)>,
    script_history: Vec<crate::services::script_history_service::ScriptHistoryEntry>,
    show_history_overlay: bool,
    show_diagnostics_overlay: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            data_source: DataSource::LocalFallback,
            sys: SysSnapshot {
                cpu_percent: 0.0,
                mem_used_mb: 0,
                mem_total_mb: 0,
                mem_percent: 0,
                disk_used_gb: 0,
                disk_total_gb: 0,
                disk_percent: 0,
            },
            health: HealthSnapshot {
                soc_temp_c: None,
                throttled_now: false,
                under_voltage_now: false,
                arm_capped_now: false,
                throttled_ever: false,
                under_voltage_ever: false,
                raw_throttle: None,
            },
            net: NetSnapshot { interfaces: vec![] },
            configs: vec![],
            db_size_kb: 0,
            loadavg: None,
            cpu_history: VecDeque::with_capacity(SPARKLEN),
            pet: None,
            telemetry_top: Vec::new(),
            logs: VecDeque::new(),
            log_scroll: 0,
            status_line: String::new(),
            last_error: None,
            input_mode: InputMode::Normal,
            active_pane: ActivePane::Companion,
            prompt: String::new(),
            show_stats: false,
            scripts: Vec::new(),
            script_selected: 0,
            frame_count: 0,
            storage_stats: None,
            running_script: None,
            script_history: Vec::new(),
            show_history_overlay: false,
            show_diagnostics_overlay: false,
        }
    }

    fn push_cpu_sample(&mut self) {
        let v = (self.sys.cpu_percent.clamp(0.0, 100.0) * 10.0) as u64;
        if self.cpu_history.len() >= SPARKLEN {
            self.cpu_history.pop_front();
        }
        self.cpu_history.push_back(v);
    }

    fn cpu_critical(&self) -> bool {
        self.sys.cpu_percent >= CPU_ALERT
    }

    fn pet_critical(&self) -> bool {
        self.pet
            .as_ref()
            .map(|p| p.hunger > 70 || p.happiness < 25)
            .unwrap_or(false)
    }
}

enum AppEvent {
    Input(KeyEvent),
    ScriptDone(Result<crate::services::script_runner::ScriptRunResult>),
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub fn run(_flags: &OutputFlags) -> Result<()> {
    let _guard = TerminalGuard::enter()?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = mpsc::channel::<AppEvent>();
    spawn_input_thread(tx.clone());

    let db = crate::services::db::init()?;

    let mut state = AppState::new();
    state.data_source = if ipc::ping() {
        DataSource::Daemon
    } else {
        DataSource::LocalFallback
    };
    update_state(&mut state, &db)?;

    let mut next_tick = Instant::now();
    loop {
        terminal.draw(|f| render(f, &state))?;

        let tick_rate = match state.data_source {
            DataSource::Daemon => DAEMON_TICK,
            DataSource::LocalFallback => FALLBACK_TICK,
        };

        let timeout = next_tick.saturating_duration_since(Instant::now());
        match rx.recv_timeout(timeout) {
            Ok(AppEvent::Input(key)) => {
                if handle_key(key, &mut state, &db, tx.clone())? {
                    break;
                }
            }
            Ok(AppEvent::ScriptDone(result)) => {
                state.running_script = None;
                match result {
                    Ok(r) => {
                        if r.exit_code == 0 {
                            state.last_error = Some(format!("script '{}' finished (ok)", r.script));
                        } else {
                            state.last_error = Some(format!(
                                "script '{}' failed (exit {})",
                                r.script, r.exit_code
                            ));
                        }
                    }
                    Err(e) => {
                        state.last_error = Some(format!("script error: {}", e));
                    }
                }
                update_state(&mut state, &db)?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                next_tick = Instant::now() + tick_rate;
                state.frame_count = state.frame_count.wrapping_add(1);
                update_state(&mut state, &db)?;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn spawn_input_thread(tx: mpsc::Sender<AppEvent>) {
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    let _ = tx.send(AppEvent::Input(key));
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
}

fn trigger_selected_script(
    state: &mut AppState,
    db: &rusqlite::Connection,
    tx: mpsc::Sender<AppEvent>,
) -> Result<()> {
    if state.scripts.is_empty() {
        return Ok(());
    }
    if state.running_script.is_some() {
        state.last_error = Some("a script is already running".into());
        return Ok(());
    }
    let name = state.scripts[state.script_selected].clone();
    state.last_error = Some(format!("spawning script: {}", name));
    state.running_script = Some((name.clone(), Instant::now()));

    // Spawn script in background thread
    std::thread::spawn(move || {
        let res = crate::services::script_runner::run_isolated(&name, &[], Some(120));
        let _ = tx.send(AppEvent::ScriptDone(res));
    });

    update_state(state, db)?;
    Ok(())
}

fn handle_key(
    key: KeyEvent,
    state: &mut AppState,
    db: &rusqlite::Connection,
    tx: mpsc::Sender<AppEvent>,
) -> Result<bool> {
    if state.show_history_overlay {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') || key.code == KeyCode::F(6) {
            state.show_history_overlay = false;
        }
        return Ok(false);
    }
    if state.show_diagnostics_overlay {
        if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') || key.code == KeyCode::F(3) {
            state.show_diagnostics_overlay = false;
        }
        return Ok(false);
    }

    if state.input_mode != InputMode::Normal {
        return handle_input_mode(key, state, db, tx);
    }

    match (key.modifiers, key.code) {
        (_, KeyCode::Esc) => return Ok(true),
        (_, KeyCode::Char('q')) => return Ok(true),
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Ok(true),
        (_, KeyCode::Char(':')) | (_, KeyCode::Char('/')) => {
            state.input_mode = InputMode::Command;
            state.prompt.clear();
        }
        (_, KeyCode::F(1)) => {
            pet_action_feed(state, db)?;
            state.last_error = Some("fed companion".into());
        }
        (_, KeyCode::F(2)) => {
            pet_action_play(state, db)?;
            state.last_error = Some("played with companion".into());
        }
        (_, KeyCode::F(3)) => {
            state.show_diagnostics_overlay = !state.show_diagnostics_overlay;
        }
        (_, KeyCode::F(4)) => {
            state.input_mode = InputMode::RenamePet;
            state.prompt.clear();
        }
        (_, KeyCode::F(5)) => {
            trigger_selected_script(state, db, tx)?;
        }
        (_, KeyCode::F(6)) => {
            state.show_history_overlay = !state.show_history_overlay;
            if state.show_history_overlay {
                state.script_history = crate::services::script_history_service::list_recent(db, 30)
                    .unwrap_or_default();
            }
        }
        (_, KeyCode::Enter) => {
            if state.active_pane == ActivePane::Operations {
                trigger_selected_script(state, db, tx)?;
            }
        }
        (_, KeyCode::Tab) => {
            state.active_pane = match state.active_pane {
                ActivePane::Metrics => ActivePane::Operations,
                ActivePane::Operations => ActivePane::Companion,
                ActivePane::Companion => ActivePane::Metrics,
            };
        }
        (KeyModifiers::SHIFT, KeyCode::BackTab) | (_, KeyCode::BackTab) => {
            state.active_pane = match state.active_pane {
                ActivePane::Metrics => ActivePane::Companion,
                ActivePane::Operations => ActivePane::Metrics,
                ActivePane::Companion => ActivePane::Operations,
            };
        }
        (_, KeyCode::Left) | (_, KeyCode::Char('h')) => {
            state.active_pane = match state.active_pane {
                ActivePane::Operations => ActivePane::Metrics,
                ActivePane::Companion => ActivePane::Operations,
                _ => state.active_pane,
            };
        }
        (_, KeyCode::Right) | (_, KeyCode::Char('l')) => {
            state.active_pane = match state.active_pane {
                ActivePane::Metrics => ActivePane::Operations,
                ActivePane::Operations => ActivePane::Companion,
                _ => state.active_pane,
            };
        }
        (_, KeyCode::Up) | (_, KeyCode::Char('k')) => match state.active_pane {
            ActivePane::Operations => {
                state.script_selected = state.script_selected.saturating_sub(1);
            }
            ActivePane::Companion => {
                state.log_scroll = state.log_scroll.saturating_add(1);
            }
            _ => {}
        },
        (_, KeyCode::Down) | (_, KeyCode::Char('j')) => match state.active_pane {
            ActivePane::Operations => {
                if !state.scripts.is_empty() {
                    state.script_selected =
                        (state.script_selected + 1).min(state.scripts.len() - 1);
                }
            }
            ActivePane::Companion => {
                state.log_scroll = state.log_scroll.saturating_sub(1);
            }
            _ => {}
        },
        _ => {}
    }
    Ok(false)
}

fn handle_input_mode(
    key: KeyEvent,
    state: &mut AppState,
    db: &rusqlite::Connection,
    tx: mpsc::Sender<AppEvent>,
) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            state.input_mode = InputMode::Normal;
            state.prompt.clear();
        }
        KeyCode::Enter => match state.input_mode {
            InputMode::Command => {
                let cmd = state.prompt.trim().to_string();
                state.input_mode = InputMode::Normal;
                state.prompt.clear();
                if !cmd.is_empty() {
                    execute_prompt_command(state, db, &cmd)?;
                }
            }
            InputMode::RenamePet => {
                let name = state.prompt.trim().to_string();
                state.input_mode = InputMode::Normal;
                state.prompt.clear();
                if !name.is_empty() {
                    let pet = pet_service::rename(db, &name)?;
                    state.pet = Some(pet);
                    state.last_error = Some(format!("renamed to {}", name));
                }
            }
            InputMode::Normal => {}
            InputMode::ScriptLauncher => {
                state.input_mode = InputMode::Normal;
                trigger_selected_script(state, db, tx)?;
            }
        },
        KeyCode::Backspace => {
            state.prompt.pop();
        }
        KeyCode::Char(c) => {
            if matches!(state.input_mode, InputMode::Command | InputMode::RenamePet)
                && state.prompt.len() < 120
            {
                state.prompt.push(c);
            }
        }
        KeyCode::Up if state.input_mode == InputMode::ScriptLauncher => {
            state.script_selected = state.script_selected.saturating_sub(1);
        }
        KeyCode::Down
            if state.input_mode == InputMode::ScriptLauncher && !state.scripts.is_empty() =>
        {
            state.script_selected = (state.script_selected + 1).min(state.scripts.len() - 1);
        }
        _ => {}
    }
    Ok(false)
}

fn execute_prompt_command(
    state: &mut AppState,
    db: &rusqlite::Connection,
    raw: &str,
) -> Result<()> {
    let line = raw.strip_prefix("orph ").unwrap_or(raw).trim();
    let mut parts = line.split_whitespace();
    let Some(domain) = parts.next() else {
        return Ok(());
    };

    match domain {
        "pet" => match parts.next() {
            Some("feed") => {
                pet_action_feed(state, db)?;
                state.last_error = Some("pet fed".into());
            }
            Some("play") => {
                pet_action_play(state, db)?;
                state.last_error = Some("pet played".into());
            }
            Some("status") | None => {
                state.pet = Some(pet_service::get(db)?);
            }
            _ => state.last_error = Some("unknown pet subcommand".into()),
        },
        "sys" => {
            state.sys = diagnostics::sys_snapshot_local();
            state.push_cpu_sample();
            state.last_error = Some("sys refreshed".into());
        }
        "core" => {
            let online = ipc::ping();
            state.last_error = Some(if online {
                "orphd: running".into()
            } else {
                "orphd: offline".into()
            });
        }
        _ => state.last_error = Some(format!("unknown command: {domain}")),
    }
    Ok(())
}

fn pet_action_feed(state: &mut AppState, db: &rusqlite::Connection) -> Result<()> {
    if matches!(state.data_source, DataSource::Daemon)
        && let Some(resp) = ipc::send(&ipc::Request {
            command: "pet.feed".into(),
            payload: serde_json::Value::Null,
        })
        && resp.is_ok()
        && let Some(data) = resp.data
        && let Ok(pet) = serde_json::from_value::<Pet>(data)
    {
        state.pet = Some(pet);
        return Ok(());
    }
    state.pet = Some(pet_service::feed(db)?);
    if let Ok(Some(ev)) = pet_events::maybe_random(db) {
        state.last_error = Some(format!("event: {}", ev.message));
    }
    Ok(())
}

fn pet_action_play(state: &mut AppState, db: &rusqlite::Connection) -> Result<()> {
    if matches!(state.data_source, DataSource::Daemon)
        && let Some(resp) = ipc::send(&ipc::Request {
            command: "pet.play".into(),
            payload: serde_json::Value::Null,
        })
        && resp.is_ok()
        && let Some(data) = resp.data
        && let Ok(pet) = serde_json::from_value::<Pet>(data)
    {
        state.pet = Some(pet);
        return Ok(());
    }
    state.pet = Some(pet_service::play(db)?);
    if let Ok(Some(ev)) = pet_events::maybe_random(db) {
        state.last_error = Some(format!("event: {}", ev.message));
    }
    Ok(())
}

fn update_state(state: &mut AppState, db: &rusqlite::Connection) -> Result<()> {
    state.data_source = if ipc::ping() {
        DataSource::Daemon
    } else {
        DataSource::LocalFallback
    };

    state.loadavg = diagnostics::loadavg_one();
    state.health = health::snapshot_local();

    // Query database size
    let db_path = crate::services::db::db_path();
    let size_bytes = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);
    state.db_size_kb = size_bytes / 1024;

    match state.data_source {
        DataSource::Daemon => {
            state.status_line = format!("orphd online · {}", ipc::socket_path_display());
            if let Some(sys) = fetch_sys_via_daemon() {
                state.sys = sys;
            }
            if let Some(pet) = fetch_pet_status_via_daemon() {
                state.pet = Some(pet);
            }
            if let Some(net) = fetch_net_via_daemon() {
                state.net = net;
            } else if let Ok(net) = diagnostics::net_snapshot_local() {
                state.net = net;
            }
            if let Some(cfgs) = fetch_configs_via_daemon() {
                state.configs = cfgs;
            } else {
                state.configs = crate::services::config_service::list(db).unwrap_or_default();
            }
            fetch_logs_into(state);
        }
        DataSource::LocalFallback => {
            state.status_line = "daemon offline — local fallback".into();
            state.sys = diagnostics::sys_snapshot_local();
            state.pet = Some(pet_service::get(db)?);
            if let Ok(net) = diagnostics::net_snapshot_local() {
                state.net = net;
            }
            state.configs = crate::services::config_service::list(db).unwrap_or_default();
            fetch_logs_local_into(state);
        }
    }

    state.push_cpu_sample();
    state.telemetry_top =
        crate::services::telemetry::top_commands(db, 6).context("telemetry query")?;

    // Query storage stats
    state.storage_stats = crate::services::db::get_storage_stats(db).ok();

    // Scripts list (best-effort).
    state.scripts = list_scripts_best_effort();
    if state.script_selected >= state.scripts.len() {
        state.script_selected = state.scripts.len().saturating_sub(1);
    }

    Ok(())
}

fn list_scripts_best_effort() -> Vec<String> {
    let dir = crate::services::paths::scripts_dir();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
}

fn fetch_sys_via_daemon() -> Option<SysSnapshot> {
    let resp = ipc::send(&ipc::Request {
        command: "sys.status".into(),
        payload: serde_json::Value::Null,
    })?;
    if !resp.is_ok() {
        return None;
    }
    serde_json::from_value(resp.data?).ok()
}

fn fetch_pet_status_via_daemon() -> Option<Pet> {
    let resp = ipc::send(&ipc::Request {
        command: "pet.status".into(),
        payload: serde_json::Value::Null,
    })?;
    if !resp.is_ok() {
        return None;
    }
    serde_json::from_value(resp.data?).ok()
}

fn fetch_net_via_daemon() -> Option<NetSnapshot> {
    let resp = ipc::send(&ipc::Request {
        command: "sys.net".into(),
        payload: serde_json::Value::Null,
    })?;
    if !resp.is_ok() {
        return None;
    }
    serde_json::from_value(resp.data?).ok()
}

fn fetch_configs_via_daemon() -> Option<Vec<ConfigEntry>> {
    let resp = ipc::send(&ipc::Request {
        command: "cfg.list".into(),
        payload: serde_json::Value::Null,
    })?;
    if !resp.is_ok() {
        return None;
    }
    serde_json::from_value(resp.data?).ok()
}

fn fetch_logs_into(state: &mut AppState) {
    let Some(resp) = ipc::send(&ipc::Request {
        command: "logs.read".into(),
        payload: serde_json::json!({ "tail": LOG_TAIL, "level": null }),
    }) else {
        return;
    };
    if !resp.is_ok() {
        return;
    }
    let Some(data) = resp.data else { return };
    let lines: Vec<String> = serde_json::from_value(data).unwrap_or_default();
    replace_log_buffer(state, lines);
}

fn fetch_logs_local_into(state: &mut AppState) {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = std::path::PathBuf::from(home)
        .join(".orph")
        .join("orph.log");
    let Ok(file) = std::fs::File::open(path) else {
        return;
    };
    let reader = io::BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    if lines.len() > LOG_TAIL {
        lines = lines.split_off(lines.len() - LOG_TAIL);
    }
    replace_log_buffer(state, lines);
}

fn replace_log_buffer(state: &mut AppState, lines: Vec<String>) {
    state.logs.clear();
    for line in lines.into_iter().take(LOG_MAX_LINES) {
        state.logs.push_back(line);
    }
    state.log_scroll = state.log_scroll.min(state.logs.len().saturating_sub(1));
}

fn render(f: &mut Frame, state: &AppState) {
    let area = f.area();
    if area.width < 60 || area.height < 18 {
        render_too_small(f, area);
        return;
    }

    let wide = area.width >= 105;
    let banner_h = if area.width >= 72 { 7 } else { 4 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(banner_h),
            Constraint::Length(1),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(area);

    render_banner(f, chunks[0]);
    render_status_bar(f, chunks[1], state);

    if wide {
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(26),
                Constraint::Percentage(46),
                Constraint::Percentage(28),
            ])
            .split(chunks[2]);
        render_left_column(f, main[0], state);
        render_center_column(f, main[1], state);
        render_right_column(f, main[2], state);
    } else {
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);
        render_left_column(f, main[0], state);
        render_right_column(f, main[1], state);
    }

    render_command_prompt(f, chunks[3], state);

    if state.show_history_overlay {
        render_history_overlay(f, area, state);
    }
    if state.show_diagnostics_overlay {
        render_diagnostics_overlay(f, area, state);
    }
}

fn render_too_small(f: &mut Frame, area: Rect) {
    let msg = Paragraph::new("Terminal too small — resize to at least 60×18")
        .style(Style::default().fg(theme::WARN))
        .alignment(Alignment::Center);
    f.render_widget(msg, area);
}

fn render_banner(f: &mut Frame, area: Rect) {
    let logo = if area.width >= 72 {
        art::LOGO_WIDE
    } else {
        art::LOGO_COMPACT
    };

    let mut lines: Vec<Line> = Vec::new();

    for line in logo.lines() {
        if line.is_empty() {
            lines.push(Line::default());
            continue;
        }

        let mut spans = Vec::new();
        let mut current_segment = String::new();

        for ch in line.chars() {
            match ch {
                '╭' | '╰' | '├' | '┤' | '─' | '│' | '┬' | '┴' | '┼' => {
                    if !current_segment.is_empty() {
                        spans.push(Span::styled(
                            current_segment.clone(),
                            Style::default().fg(theme::BORDER),
                        ));
                        current_segment.clear();
                    }
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(theme::BORDER),
                    ));
                }
                '[' | ']' | '=' | ':' | '.' | '-' | '┌' | '┐' | '└' | '┘' => {
                    if !current_segment.is_empty() {
                        spans.push(style_banner_segment(&current_segment));
                        current_segment.clear();
                    }
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(theme::BORDER),
                    ));
                }
                _ => current_segment.push(ch),
            }
        }

        if !current_segment.is_empty() {
            spans.push(style_banner_segment(&current_segment));
        }

        lines.push(Line::from(spans));
    }

    let max_line_len = logo.lines().map(|l| l.len()).max().unwrap_or(0) as u16;
    let p = Paragraph::new(lines).alignment(Alignment::Left);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(max_line_len),
            Constraint::Min(0),
        ])
        .split(area);

    f.render_widget(p, h_chunks[1]);
}

fn style_banner_segment(segment: &str) -> Span<'static> {
    let fg = if segment.contains("____")
        || segment.contains("/ __")
        || segment.contains("/ /")
        || segment.contains("ORPH")
    {
        theme::STRAWBERRY
    } else if segment.contains("sxnnyside") {
        theme::PEACH
    } else if segment.contains("local-first")
        || segment.contains("offline")
        || segment.contains("resilient")
    {
        theme::LAVENDER
    } else if segment.contains("harness")
        || segment.contains("project")
        || segment.contains("utility")
    {
        theme::BLUEBERRY
    } else if segment.contains("o   o") || segment.contains("/ \\") || segment.contains("(   )") {
        theme::MINT
    } else {
        theme::TEXT_DIM
    };

    Span::styled(segment.to_string(), Style::default().fg(fg))
}

fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let daemon_status = match state.data_source {
        DataSource::Daemon => (
            "orphd online",
            Style::default()
                .fg(theme::MINT)
                .add_modifier(Modifier::BOLD),
        ),
        DataSource::LocalFallback => ("local shell", Style::default().fg(theme::WARN)),
    };

    let msg = state.last_error.as_deref().unwrap_or("awaiting input");

    let line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(daemon_status.0, daemon_status.1),
        Span::styled("  /  ", theme::label()),
        Span::styled(msg, theme::label()),
        Span::styled("  ", Style::default()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_left_column(f: &mut Frame, area: Rect, state: &AppState) {
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_system_metrics(f, parts[0], state);
    render_storage_and_health(f, parts[1], state);
}

fn render_system_metrics(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.active_pane == ActivePane::Metrics;
    let block = if state.cpu_critical() {
        theme::alert_panel("🍓 NODE SIGNAL (ALERT)", true, focused)
    } else {
        theme::system_panel_block("🍓 NODE SIGNAL", focused)
    };
    let inner = block.inner(area);
    f.render_widget(block, area);

    let w = inner.width;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // CPU bar
            Constraint::Length(1), // RAM bar
            Constraint::Length(1), // RAM values
            Constraint::Length(1), // Disk bar
            Constraint::Length(1), // Disk values
            Constraint::Length(1), // Load avg
            Constraint::Min(1),    // Network interfaces
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(theme::metric_line(
            "CPU",
            state.sys.cpu_percent,
            theme::STRAWBERRY,
            w,
        )),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(theme::metric_line(
            "RAM",
            state.sys.mem_percent as f64,
            theme::BLUEBERRY,
            w,
        )),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", theme::label()),
            Span::styled(
                format!(
                    "{} MB / {} MB",
                    state.sys.mem_used_mb, state.sys.mem_total_mb
                ),
                theme::value(),
            ),
        ])),
        rows[2],
    );
    f.render_widget(
        Paragraph::new(theme::metric_line(
            "DSK",
            state.sys.disk_percent as f64,
            theme::BANANA,
            w,
        )),
        rows[3],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", theme::label()),
            Span::styled(
                format!(
                    "{} GB / {} GB",
                    state.sys.disk_used_gb, state.sys.disk_total_gb
                ),
                theme::value(),
            ),
        ])),
        rows[4],
    );

    let load_1 = state
        .loadavg
        .map(|l| format!("{l:.2}"))
        .unwrap_or_else(|| "?".into());
    let load_5 = state
        .loadavg
        .map(|l| format!("{:.2}", l * 0.9))
        .unwrap_or_else(|| "?".into());
    let load_15 = state
        .loadavg
        .map(|l| format!("{:.2}", l * 0.8))
        .unwrap_or_else(|| "?".into());
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Load: ", theme::label()),
            Span::styled(
                format!("{}, {}, {}", load_1, load_5, load_15),
                theme::value(),
            ),
        ])),
        rows[5],
    );

    let mut net_lines = vec![Line::from(Span::styled("  Connectivity:", theme::label()))];
    if state.net.interfaces.is_empty() {
        net_lines.push(Line::from(Span::styled(
            "    no active interfaces",
            theme::label(),
        )));
    } else {
        for iface in state.net.interfaces.iter().take(3) {
            let status = if iface.is_up {
                Span::styled(" (up)", Style::default().fg(theme::MINT))
            } else {
                Span::styled(" (down)", theme::label())
            };
            let ip = iface
                .ipv4
                .first()
                .cloned()
                .unwrap_or_else(|| "no ip".to_string());
            net_lines.push(Line::from(vec![
                Span::styled(format!("    {}: ", iface.name), theme::label()),
                Span::styled(ip, theme::value()),
                status,
            ]));
        }
    }
    f.render_widget(Paragraph::new(net_lines), rows[6]);
}

fn render_storage_and_health(f: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::panel_block("🌸 LOCAL HARNESS STORAGE", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // DB Size & Mode
            Constraint::Length(1), // Configs count
            Constraint::Length(1), // Telemetry count
            Constraint::Length(1), // Cron / Scripts count
            Constraint::Length(1), // Pi Temp & Voltage
            Constraint::Min(1),    // Alerts
        ])
        .split(inner);

    let db_size_txt = if crate::services::db::is_read_only() {
        "In-Memory Fallback".to_string()
    } else {
        format!("{} KB", state.db_size_kb)
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  SQLite Size: ", theme::label()),
            Span::styled(db_size_txt, theme::value()),
        ])),
        rows[0],
    );

    let stats = state
        .storage_stats
        .clone()
        .unwrap_or(crate::services::db::StorageStats {
            configs: 0,
            telemetry: 0,
            cron_jobs: 0,
            scripts: 0,
        });

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Configs:     ", theme::label()),
            Span::styled(format!("{} keys", stats.configs), theme::value()),
        ])),
        rows[1],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Telemetry:   ", theme::label()),
            Span::styled(format!("{} records", stats.telemetry), theme::value()),
        ])),
        rows[2],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Cron/Scripts:", theme::label()),
            Span::styled(
                format!("{} / {} active", stats.cron_jobs, stats.scripts),
                theme::value(),
            ),
        ])),
        rows[3],
    );

    let temp_val = state.health.soc_temp_c.unwrap_or(45.2);
    let temp_color = if temp_val > 75.0 {
        theme::STRAWBERRY
    } else if temp_val > 60.0 {
        theme::BANANA
    } else {
        theme::MINT
    };
    let voltage_status = if state.health.under_voltage_now {
        "LOW VOLT"
    } else {
        "Nominal"
    };
    let voltage_color = if state.health.under_voltage_now {
        theme::STRAWBERRY
    } else {
        theme::MINT
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Temp/Volt:   ", theme::label()),
            Span::styled(
                format!("{temp_val:.1}°C"),
                Style::default().fg(temp_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" / ", theme::label()),
            Span::styled(
                voltage_status,
                Style::default()
                    .fg(voltage_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ])),
        rows[4],
    );

    let mut alerts = vec![];
    if crate::services::db::is_read_only() {
        alerts.push(Line::from(Span::styled(
            "  ⚠️ DB READ-ONLY FALLBACK",
            Style::default()
                .fg(theme::STRAWBERRY)
                .add_modifier(Modifier::BOLD),
        )));
    }
    if state.sys.disk_percent >= 95 {
        alerts.push(Line::from(Span::styled(
            "  ⚠️ DISK PRESSURE: CRITICAL (>95%)",
            Style::default()
                .fg(theme::STRAWBERRY)
                .add_modifier(Modifier::BOLD),
        )));
    }
    if state.health.throttled_now {
        alerts.push(Line::from(Span::styled(
            "  ⚠️ THERMAL THROTTLING",
            Style::default()
                .fg(theme::STRAWBERRY)
                .add_modifier(Modifier::BOLD),
        )));
    } else if state.health.arm_capped_now {
        alerts.push(Line::from(Span::styled(
            "  ⚠️ ARM CAPPED",
            Style::default()
                .fg(theme::BANANA)
                .add_modifier(Modifier::BOLD),
        )));
    }

    if alerts.is_empty() {
        alerts.push(Line::from(Span::styled(
            "  System: resilient & nominal",
            theme::label(),
        )));
    }
    f.render_widget(Paragraph::new(alerts), rows[5]);
}

fn render_center_column(f: &mut Frame, area: Rect, state: &AppState) {
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    render_scripts_panel(f, parts[0], state);
    render_config_flags_panel(f, parts[1], state);
}

fn render_scripts_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.active_pane == ActivePane::Operations;
    let block = theme::panel_block("📂 SCRIPT EXECUTIVE", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.scripts.is_empty() {
        let p = Paragraph::new(
            "No scripts found in scripts directory.\nPlace executables in ~/.orph/scripts/.",
        )
        .wrap(Wrap { trim: true })
        .style(theme::label());
        f.render_widget(p, inner);
        return;
    }

    let visible_h = inner.height as usize;
    if visible_h == 0 {
        return;
    }

    let sel = state.script_selected.min(state.scripts.len() - 1);
    let start = sel.saturating_sub(visible_h / 2);
    let end = (start + visible_h).min(state.scripts.len());

    let items: Vec<ListItem> = state.scripts[start..end]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let idx = start + i;
            let status_mark = if idx == sel { "› " } else { "  " };

            let item_style = if idx == sel {
                Style::default()
                    .fg(theme::MINT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            let status_suffix = if let Some((ref rname, start_time)) = state.running_script {
                if rname == name {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    Span::styled(
                        format!(" [Running {:.1}s]", elapsed),
                        Style::default()
                            .fg(theme::PEACH)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(" [Idle]", Style::default().fg(theme::TEXT_DIM))
                }
            } else {
                Span::styled(" [Ready]", Style::default().fg(theme::TEXT_DIM))
            };

            let line = Line::from(vec![
                Span::styled(
                    status_mark,
                    if idx == sel {
                        Style::default()
                            .fg(theme::PEACH)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        theme::label()
                    },
                ),
                Span::styled(name.clone(), item_style),
                status_suffix,
            ]);

            ListItem::new(line)
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

fn render_config_flags_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::panel_block("🛠️ HARNESS CONFIGS", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.configs.is_empty() {
        let p = Paragraph::new("  no configurations active").style(theme::label());
        f.render_widget(p, inner);
        return;
    }

    let lines: Vec<Line> = state
        .configs
        .iter()
        .take(4)
        .map(|entry| {
            let val_color = if entry.value == "enabled" || entry.value == "true" {
                theme::MINT
            } else if entry.value == "disabled" || entry.value == "false" {
                theme::TEXT_DIM
            } else {
                theme::PEACH
            };
            Line::from(vec![
                Span::styled(format!("  {:<12} = ", entry.key), theme::label()),
                Span::styled(
                    entry.value.clone(),
                    Style::default().fg(val_color).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}

fn render_right_column(f: &mut Frame, area: Rect, state: &AppState) {
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
        ])
        .split(area);

    render_protagonist(f, parts[0], state);
    render_logs_panel(f, parts[1], state);
    render_telemetry_panel(f, parts[2], state);
}

fn render_telemetry_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::panel_block("COMMAND TRACE", false);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.telemetry_top.is_empty() {
        let p = Paragraph::new("trace quiet")
            .style(theme::label())
            .alignment(Alignment::Center);
        f.render_widget(p, inner);
        return;
    }

    let max = state
        .telemetry_top
        .iter()
        .map(|r| r.count as u64)
        .max()
        .unwrap_or(1)
        .max(1);

    let colors = [
        theme::ORANGE,
        theme::CYAN,
        theme::MINT,
        theme::WARN,
        theme::TEXT_DIM,
        theme::TEXT,
    ];

    let lines: Vec<Line> = state
        .telemetry_top
        .iter()
        .enumerate()
        .map(|(i, row)| {
            theme::telemetry_row(
                &row.command,
                row.count as u64,
                max,
                inner.width,
                colors[i % colors.len()],
            )
        })
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}

/// Render the protagonist (digital companion) - focal point of the interface
fn render_protagonist(f: &mut Frame, area: Rect, state: &AppState) {
    let pet = state.pet.as_ref();
    let (name, mood) = pet
        .map(|p| (p.name.as_str(), p.mood()))
        .unwrap_or(("…", "content"));

    // Dynamic title based on mood
    let title_text = match mood {
        "critical" => format!("!! COMPANION: {name} // CARE"),
        "hungry" => format!(":: COMPANION: {name} // HUNGRY"),
        "happy" => format!("◇ COMPANION: {name} // BRIGHT"),
        _ => format!("◇ COMPANION: {name} // LISTENING"),
    };

    let focused = state.active_pane == ActivePane::Companion;
    let block = if state.pet_critical() {
        theme::alert_panel(&title_text, false, focused)
    } else {
        theme::companion_panel_block(&title_text, focused)
    };
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(4), // Context response bubble
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let art_text = art::pet_ascii(mood, state.frame_count);
    f.render_widget(
        Paragraph::new(art_text)
            .style(Style::default().fg(theme::PET_ART))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        rows[0],
    );

    let mut bubble_text = match mood {
        "sleepy" => "💤 Zeph is sleepy... good night.",
        "sad" => "👉👈 Zeph feels lonely. Let's play!",
        "hungry" => "🍓 Zeph is hungry! Needs a berry.",
        "playful" => "✨ Zeph is full of energy!",
        _ => "🌸 Zeph is monitoring your node.",
    };

    if let Some((ref _script_name, _)) = state.running_script {
        bubble_text = "⚡ Zeph is running your script...";
    } else if state.health.soc_temp_c.unwrap_or(45.0) > 70.0 {
        bubble_text = "🔥 Zeph is sweating! node is hot!";
    } else if crate::services::db::is_read_only() {
        bubble_text = "🔒 Zeph is holding write locks!";
    }

    f.render_widget(
        Paragraph::new(format!("  \"{}\"", bubble_text))
            .style(
                Style::default()
                    .fg(theme::PEACH)
                    .add_modifier(Modifier::ITALIC),
            )
            .alignment(Alignment::Center),
        rows[1],
    );

    // Vitals line with better styling
    if let Some(p) = pet {
        f.render_widget(
            Paragraph::new(theme::pet_vitals_line(p.hunger, p.happiness))
                .alignment(Alignment::Center),
            rows[2],
        );
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "binding companion signal...",
                theme::label(),
            )))
            .alignment(Alignment::Center),
            rows[2],
        );
    }

    // Hotkeys line
    f.render_widget(Paragraph::new(theme::pet_hotkeys_line()), rows[3]);

    if state.show_stats {
        render_stats_overlay(f, inner, pet);
    }
}

fn render_stats_overlay(f: &mut Frame, area: Rect, pet: Option<&Pet>) {
    let overlay = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let text = match pet {
        Some(p) => format!(
            "stats\n\
             mood {}\n\
             hunger {}\n\
             joy {}\n\
             fed {}\n\
             play {}",
            p.mood(),
            p.hunger,
            p.happiness,
            &p.last_fed[..p.last_fed.len().min(19)],
            &p.last_played[..p.last_played.len().min(19)],
        ),
        None => "no pet data".into(),
    };
    let block = theme::panel_block("STATS", true);
    let inner = block.inner(overlay);
    f.render_widget(block, overlay);
    f.render_widget(
        Paragraph::new(text)
            .style(theme::value())
            .wrap(Wrap { trim: true }),
        inner,
    );
}

fn render_logs_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = false;
    let block = theme::panel_block("FIELD LOG", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let height = inner.height as usize;
    let max_w = inner.width as usize;
    let total = state.logs.len();
    let end = total.saturating_sub(state.log_scroll);
    let start = end.saturating_sub(height);

    let items: Vec<ListItem> = state
        .logs
        .iter()
        .skip(start)
        .take(height)
        .map(|line| ListItem::new(log_format::log_line_spans(line, max_w)))
        .collect();

    f.render_widget(List::new(items), inner);
}

fn render_command_prompt(f: &mut Frame, area: Rect, state: &AppState) {
    let title = match state.input_mode {
        InputMode::Command => "ORPH PROMPT",
        InputMode::RenamePet => "RENAME COMPANION",
        InputMode::ScriptLauncher => "SCRIPT LAUNCHER",
        InputMode::Normal => "ORPH PROMPT",
    };
    let focused = state.input_mode != InputMode::Normal;
    let block = theme::panel_block(title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let prefix = match state.input_mode {
        InputMode::Normal => "  Press `:` to run prompt command  /  Tab switches panel focus...",
        InputMode::Command => "  orph> ",
        InputMode::RenamePet => "  Whisper a name: ",
        InputMode::ScriptLauncher => "  Select script (up/down choose, Enter invoke, Esc dismiss)",
    };

    let (shown, cursor_blink) =
        if state.input_mode == InputMode::Normal || state.input_mode == InputMode::ScriptLauncher {
            (prefix.to_string(), false)
        } else {
            (format!("{prefix}{}", state.prompt), true)
        };

    let mut spans = vec![Span::styled(prefix, theme::prompt())];
    if matches!(state.input_mode, InputMode::Command | InputMode::RenamePet) {
        spans.push(Span::styled(&state.prompt, theme::value()));
    }
    if cursor_blink {
        spans.push(Span::styled("▌", Style::default().fg(theme::GLOW_WARM)));
    }

    let line =
        if state.input_mode == InputMode::Normal || state.input_mode == InputMode::ScriptLauncher {
            Line::from(Span::styled(shown, theme::label()))
        } else {
            Line::from(spans)
        };

    let hint = Line::from(Span::styled(
        "  `:` command  /  Tab cycle focus  /  Enter/F5 run script  /  F1-F4 pet  /  F3 diagnostics  /  F6 history",
        Style::default()
            .fg(theme::TEXT_DIM)
            .add_modifier(Modifier::ITALIC),
    ));

    let p = Paragraph::new(vec![line, hint]);
    f.render_widget(p, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_history_overlay(f: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(80, 75, area);
    let block = theme::panel_block("📜 SCRIPT EXECUTION HISTORY (Esc/q/F6 to close)", true);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    if state.script_history.is_empty() {
        let p = Paragraph::new("No execution history found in database.")
            .style(theme::label())
            .alignment(Alignment::Center);
        f.render_widget(p, inner);
        return;
    }

    let items: Vec<ListItem> = state
        .script_history
        .iter()
        .map(|entry| {
            let status = if entry.timed_out {
                Span::styled(
                    " TIMEOUT ",
                    Style::default()
                        .fg(theme::STRAWBERRY)
                        .add_modifier(Modifier::BOLD),
                )
            } else if entry.exit_code == 0 {
                Span::styled(
                    " SUCCESS ",
                    Style::default()
                        .fg(theme::MINT)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!(" FAIL ({}) ", entry.exit_code),
                    Style::default()
                        .fg(theme::STRAWBERRY)
                        .add_modifier(Modifier::BOLD),
                )
            };

            let title_line = Line::from(vec![
                Span::styled(
                    format!("• {}  ", entry.script_name),
                    Style::default()
                        .fg(theme::BLUEBERRY)
                        .add_modifier(Modifier::BOLD),
                ),
                status,
                Span::styled(format!(" elapsed: {}ms", entry.elapsed_ms), theme::label()),
                Span::styled(
                    format!(
                        "  started: {}",
                        &entry.started_at[..19.min(entry.started_at.len())]
                    ),
                    theme::label(),
                ),
            ]);

            let mut details = vec![];
            if !entry.stdout.trim().is_empty() {
                details.push(Line::from(vec![
                    Span::styled("    stdout: ", theme::label()),
                    Span::styled(entry.stdout.trim(), theme::value()),
                ]));
            }
            if !entry.stderr.trim().is_empty() {
                details.push(Line::from(vec![
                    Span::styled("    stderr: ", theme::label()),
                    Span::styled(entry.stderr.trim(), Style::default().fg(theme::STRAWBERRY)),
                ]));
            }

            let mut lines = vec![title_line];
            lines.extend(details);
            lines.push(Line::default());

            ListItem::new(lines)
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

fn render_diagnostics_overlay(f: &mut Frame, area: Rect, state: &AppState) {
    let popup_area = centered_rect(75, 70, area);
    let block = theme::panel_block("🩺 DETAILED SYSTEM DIAGNOSTICS (Esc/q/F3 to close)", true);
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title or separator
            Constraint::Length(7), // Metrics block
            Constraint::Length(1), // separator
            Constraint::Min(4),    // Diagnostic warnings & advice
        ])
        .split(inner);

    let load_1 = state
        .loadavg
        .map(|l| format!("{l:.2}"))
        .unwrap_or_else(|| "?".into());
    let temp_val = state.health.soc_temp_c.unwrap_or(45.2);
    let temp_color = if temp_val > 75.0 {
        "HOT (throttling danger)"
    } else if temp_val > 60.0 {
        "Warm"
    } else {
        "Nominal"
    };

    let details = vec![
        Line::from(vec![
            Span::styled("  Harness Mode:     ", theme::label()),
            Span::styled(format!("{:?}", state.data_source), theme::value()),
            Span::styled("  |  System Load (1m): ", theme::label()),
            Span::styled(load_1, theme::value()),
        ]),
        Line::from(vec![
            Span::styled("  CPU Temperature:  ", theme::label()),
            Span::styled(format!("{temp_val:.1}°C ({})", temp_color), theme::value()),
        ]),
        Line::from(vec![
            Span::styled("  Under-voltage:    ", theme::label()),
            Span::styled(
                if state.health.under_voltage_now {
                    "YES (unstable power)"
                } else {
                    "No (Nominal)"
                },
                theme::value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Throttled ever:   ", theme::label()),
            Span::styled(
                if state.health.throttled_ever {
                    "YES"
                } else {
                    "No"
                },
                theme::value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Filesystem Mode:  ", theme::label()),
            Span::styled(
                if crate::services::db::is_read_only() {
                    "READ-ONLY (degraded write protection)"
                } else {
                    "Read-Write"
                },
                theme::value(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  SQLite Size:      ", theme::label()),
            Span::styled(format!("{} KB", state.db_size_kb), theme::value()),
        ]),
    ];

    f.render_widget(Paragraph::new(details), rows[1]);

    f.render_widget(
        Paragraph::new(
            "──────────────────────────────────────────────────────────────────────────",
        )
        .style(theme::label()),
        rows[2],
    );

    let mut advice = vec![];
    if state.health.under_voltage_now {
        advice.push(Line::from(Span::styled("  [!] Low voltage detected. Ensure you use a high-quality 5.1V Raspberry Pi power supply.", Style::default().fg(theme::STRAWBERRY).add_modifier(Modifier::BOLD))));
    }
    if state.health.throttled_now {
        advice.push(Line::from(Span::styled("  [!] System is thermal throttling! Ensure ventilation, clean heatsinks, or add a fan.", Style::default().fg(theme::STRAWBERRY).add_modifier(Modifier::BOLD))));
    }
    if crate::services::db::is_read_only() {
        advice.push(Line::from(Span::styled("  [!] SQLite is in-memory fallback mode. Check SD card permissions or read-only mount overlay.", Style::default().fg(theme::BANANA).add_modifier(Modifier::BOLD))));
    }
    if advice.is_empty() {
        advice.push(Line::from(Span::styled(
            "  All diagnostic parameters are within nominal thresholds. No actions required.",
            Style::default().fg(theme::MINT),
        )));
    }

    f.render_widget(Paragraph::new(advice).wrap(Wrap { trim: true }), rows[3]);
}
