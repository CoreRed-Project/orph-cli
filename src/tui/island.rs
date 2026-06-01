use crate::cli::OutputFlags;
use crate::ipc;
use crate::models::diagnostics::SysSnapshot;
use crate::models::pet::Pet;
use crate::services::diagnostics;
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
enum InputMode {
    Normal,
    Command,
    RenamePet,
    ScriptLauncher,
}

struct AppState {
    data_source: DataSource,
    sys: SysSnapshot,
    loadavg: Option<f64>,
    cpu_history: VecDeque<u64>,
    pet: Option<Pet>,
    telemetry_top: Vec<CommandCount>,
    logs: VecDeque<String>,
    log_scroll: usize,
    status_line: String,
    last_error: Option<String>,
    input_mode: InputMode,
    prompt: String,
    show_stats: bool,
    scripts: Vec<String>,
    script_selected: usize,
    frame_count: u64, // For subtle animations
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
            loadavg: None,
            cpu_history: VecDeque::with_capacity(SPARKLEN),
            pet: None,
            telemetry_top: Vec::new(),
            logs: VecDeque::new(),
            log_scroll: 0,
            status_line: String::new(),
            last_error: None,
            input_mode: InputMode::Normal,
            prompt: String::new(),
            show_stats: false,
            scripts: Vec::new(),
            script_selected: 0,
            frame_count: 0,
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
    spawn_input_thread(tx);

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
                if handle_key(key, &mut state, &db)? {
                    break;
                }
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

fn handle_key(key: KeyEvent, state: &mut AppState, db: &rusqlite::Connection) -> Result<bool> {
    if state.input_mode != InputMode::Normal {
        return handle_input_mode(key, state, db);
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
            state.show_stats = !state.show_stats;
        }
        (_, KeyCode::F(4)) => {
            state.input_mode = InputMode::RenamePet;
            state.prompt.clear();
        }
        (_, KeyCode::F(5)) => {
            state.input_mode = InputMode::ScriptLauncher;
            state.script_selected = 0;
        }
        (_, KeyCode::Up) => {
            state.log_scroll = state.log_scroll.saturating_add(1);
        }
        (_, KeyCode::Down) => {
            state.log_scroll = state.log_scroll.saturating_sub(1);
        }
        _ => {}
    }
    Ok(false)
}

fn handle_input_mode(
    key: KeyEvent,
    state: &mut AppState,
    db: &rusqlite::Connection,
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
                if state.scripts.is_empty() {
                    state.input_mode = InputMode::Normal;
                    return Ok(false);
                }
                let name = state.scripts[state.script_selected].clone();
                state.input_mode = InputMode::Normal;
                state.last_error = Some(format!("running script: {}", name));
                // Local run (same process) for now; daemon-scheduled remains via `orph run cron`.
                let result = crate::services::script_runner::run_isolated(&name, &[], Some(120));
                match result {
                    Ok(r) => {
                        if r.exit_code == 0 {
                            state.last_error = Some(format!("script '{}' finished (ok)", name));
                        } else {
                            state.last_error =
                                Some(format!("script '{}' failed (exit {})", name, r.exit_code));
                        }
                    }
                    Err(e) => state.last_error = Some(format!("script '{}' error: {}", name, e)),
                }
                update_state(state, db)?;
            }
        },
        KeyCode::Backspace => {
            state.prompt.pop();
        }
        KeyCode::Char(c) => {
            if matches!(state.input_mode, InputMode::Command | InputMode::RenamePet) {
                if state.prompt.len() < 120 {
                    state.prompt.push(c);
                }
            }
        }
        KeyCode::Up => {
            if state.input_mode == InputMode::ScriptLauncher {
                state.script_selected = state.script_selected.saturating_sub(1);
            }
        }
        KeyCode::Down => {
            if state.input_mode == InputMode::ScriptLauncher {
                if !state.scripts.is_empty() {
                    state.script_selected =
                        (state.script_selected + 1).min(state.scripts.len() - 1);
                }
            }
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
    if matches!(state.data_source, DataSource::Daemon) {
        if let Some(resp) = ipc::send(&ipc::Request {
            command: "pet.feed".into(),
            payload: serde_json::Value::Null,
        }) && resp.is_ok()
            && let Some(data) = resp.data
            && let Ok(pet) = serde_json::from_value::<Pet>(data)
        {
            state.pet = Some(pet);
            return Ok(());
        }
    }
    state.pet = Some(pet_service::feed(db)?);
    if let Ok(Some(ev)) = pet_events::maybe_random(db) {
        state.last_error = Some(format!("event: {}", ev.message));
    }
    Ok(())
}

fn pet_action_play(state: &mut AppState, db: &rusqlite::Connection) -> Result<()> {
    if matches!(state.data_source, DataSource::Daemon) {
        if let Some(resp) = ipc::send(&ipc::Request {
            command: "pet.play".into(),
            payload: serde_json::Value::Null,
        }) && resp.is_ok()
            && let Some(data) = resp.data
            && let Ok(pet) = serde_json::from_value::<Pet>(data)
        {
            state.pet = Some(pet);
            return Ok(());
        }
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

    match state.data_source {
        DataSource::Daemon => {
            state.status_line = format!("orphd online · {}", ipc::socket_path_display());
            if let Some(sys) = fetch_sys_via_daemon() {
                state.sys = sys;
            }
            if let Some(pet) = fetch_pet_status_via_daemon() {
                state.pet = Some(pet);
            }
            fetch_logs_into(state);
        }
        DataSource::LocalFallback => {
            state.status_line = "daemon offline — local fallback".into();
            state.sys = diagnostics::sys_snapshot_local();
            state.pet = Some(pet_service::get(db)?);
            fetch_logs_local_into(state);
        }
    }

    state.push_cpu_sample();
    state.telemetry_top =
        crate::services::telemetry::top_commands(db, 6).context("telemetry query")?;

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
    let banner_h = if area.width >= 72 { 11 } else { 6 };

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
        // Product-first composition: brand and companion dominate; telemetry recedes.
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(52),
                Constraint::Percentage(28),
            ])
            .split(chunks[2]);
        render_system_info_compact(f, main[0], state);
        render_protagonist(f, main[1], state);
        render_right_panel(f, main[2], state);
    } else {
        // Vertical fallback: companion remains the lead visual.
        let main = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(12)])
            .split(chunks[2]);

        render_system_info_compact(f, main[0], state);
        render_protagonist(f, main[1], state);
    }

    render_command_prompt(f, chunks[3], state);

    if state.input_mode == InputMode::ScriptLauncher {
        render_script_launcher(f, area, state);
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

    // Artistic rendering with atmospheric colors
    let mut lines: Vec<Line> = Vec::new();

    for line in logo.lines() {
        if line.is_empty() {
            lines.push(Line::default());
            continue;
        }

        // Build spans with color-coded segments
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
                '[' | ']' | '=' | ':' | '.' | '-' => {
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

    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, area);
}

fn style_banner_segment(segment: &str) -> Span<'static> {
    let fg = if segment.contains("____")
        || segment.contains("/ __")
        || segment.contains("/ /")
        || segment.contains("\\____")
        || segment.contains("ORPH")
    {
        theme::GLOW_WARM
    } else if segment.contains("sxnnyside") || segment.contains("PROJECT") {
        theme::DIGITAL
    } else if segment.contains("LOCAL")
        || segment.contains("RASPBERRY")
        || segment.contains("SIGNAL")
        || segment.contains("SOUL")
    {
        theme::TEXT
    } else if segment.contains("companion") {
        theme::MINT
    } else {
        theme::ORANGE
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

/// Compact system info panel (left sidebar in wide mode)
fn render_system_info_compact(f: &mut Frame, area: Rect, state: &AppState) {
    let block = if state.cpu_critical() {
        theme::alert_panel("SIGNAL", true)
    } else {
        theme::system_panel_block("SIGNAL")
    };
    let inner = block.inner(area);
    f.render_widget(block, area);

    let w = inner.width;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(2),
        ])
        .split(inner);

    // Compact metrics
    f.render_widget(
        Paragraph::new(theme::metric_line(
            "CPU",
            state.sys.cpu_percent,
            theme::ORANGE,
            w,
        )),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(theme::metric_line(
            "RAM",
            state.sys.mem_percent as f64,
            theme::CYAN,
            w,
        )),
        rows[1],
    );
    f.render_widget(
        Paragraph::new(theme::metric_line(
            "DSK",
            state.sys.disk_percent as f64,
            theme::TEXT_DIM,
            w,
        )),
        rows[2],
    );

    // Load average line
    if rows[3].height > 0 {
        let load_txt = state
            .loadavg
            .map(|l| format!("{l:.1}"))
            .unwrap_or_else(|| "?".into());
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("◇ load {load_txt} ◇"),
                theme::label(),
            )))
            .alignment(Alignment::Center),
            rows[3],
        );
    }
}

/// Right panel: logs and activity (secondary contextual info)
fn render_right_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(area);

    render_logs_panel(f, sections[0], state);
    render_telemetry_panel(f, sections[1], state);
}

fn render_telemetry_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let block = theme::panel_block("COMMAND TRACE");
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

    let block = if state.pet_critical() {
        theme::alert_panel(&title_text, false)
    } else {
        theme::companion_panel_block(&title_text)
    };
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(14),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let art_text = art::pet_ascii(mood);
    f.render_widget(
        Paragraph::new(art_text)
            .style(Style::default().fg(theme::PET_ART))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        rows[0],
    );

    // Vitals line with better styling
    if let Some(p) = pet {
        f.render_widget(
            Paragraph::new(theme::pet_vitals_line(p.hunger, p.happiness))
                .alignment(Alignment::Center),
            rows[1],
        );
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "binding companion signal...",
                theme::label(),
            )))
            .alignment(Alignment::Center),
            rows[1],
        );
    }

    // Hotkeys line
    f.render_widget(Paragraph::new(theme::pet_hotkeys_line()), rows[2]);

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
    let block = theme::panel_block("STATS");
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
    let block = theme::panel_block("FIELD LOG");
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
        InputMode::RenamePet => "NAME RITE",
        InputMode::ScriptLauncher => "SCRIPT BAY",
        InputMode::Normal => "ORPH PROMPT",
    };
    let block = theme::panel_block(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let prefix = match state.input_mode {
        InputMode::Normal => "  press `:` to commune with your companion...",
        InputMode::Command => "  orph> ",
        InputMode::RenamePet => "  whisper a name: ",
        InputMode::ScriptLauncher => "  select script (up/down choose, Enter invoke, Esc dismiss)",
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

    let line = if state.input_mode == InputMode::Normal {
        Line::from(Span::styled(shown, theme::label()))
    } else if state.input_mode == InputMode::ScriptLauncher {
        Line::from(Span::styled(shown, theme::label()))
    } else {
        Line::from(spans)
    };

    let hint = Line::from(Span::styled(
        "  `:` commune  /  F1-F5 care  /  up/down observe",
        Style::default()
            .fg(theme::TEXT_DIM)
            .add_modifier(Modifier::ITALIC),
    ));

    let p = Paragraph::new(vec![line, hint]);
    f.render_widget(p, inner);
}

fn render_script_launcher(f: &mut Frame, area: Rect, state: &AppState) {
    let w = area.width.min(58);
    let h = area.height.min(18);
    let overlay = Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    };

    let block = theme::panel_block("RUN SCRIPT");
    let inner = block.inner(overlay);
    f.render_widget(block, overlay);

    if state.scripts.is_empty() {
        let p = Paragraph::new(format!(
            "no scripts in {}\nadd files and press F5",
            crate::services::paths::scripts_dir().display()
        ))
        .wrap(Wrap { trim: true })
        .style(theme::label())
        .alignment(Alignment::Left);
        f.render_widget(p, inner);
        return;
    }

    let visible_h = inner.height as usize;
    let sel = state.script_selected.min(state.scripts.len() - 1);
    let start = sel.saturating_sub(visible_h / 2);
    let end = (start + visible_h).min(state.scripts.len());

    let items: Vec<ListItem> = state.scripts[start..end]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let idx = start + i;
            let line = if idx == sel {
                Line::from(Span::styled(
                    format!("› {name}"),
                    Style::default()
                        .fg(theme::MINT)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(format!("  {name}"), theme::label()))
            };
            ListItem::new(line)
        })
        .collect();

    f.render_widget(List::new(items), inner);
}
