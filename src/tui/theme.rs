use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::block::Padding;
use ratatui::widgets::{Block, BorderType, Borders};

// ── Sxnnyside Aesthetic: Dark palette with warm digital nostalgia ─────────

pub const ORANGE: Color = Color::Rgb(226, 146, 84); // Warm phosphor amber
pub const CYAN: Color = Color::Rgb(118, 190, 204); // Aged terminal cyan
pub const MINT: Color = Color::Rgb(145, 220, 190); // Soft signal mint
pub const PET_ART: Color = Color::Rgb(194, 213, 190); // Companion bone/glow
pub const TEXT: Color = Color::Rgb(196, 198, 196); // Primary text
pub const TEXT_DIM: Color = Color::Rgb(103, 112, 118); // Subtle labels
pub const BORDER: Color = Color::Rgb(70, 94, 98); // Quiet custom frames
pub const BAR_EMPTY: Color = Color::Rgb(31, 39, 42); // Empty bar segments
pub const WARN: Color = Color::Rgb(225, 178, 94); // Warning color
pub const ERR: Color = Color::Rgb(216, 93, 96); // Error/critical color
pub const GLOW_WARM: Color = Color::Rgb(236, 176, 115); // Brand glow
pub const DIGITAL: Color = Color::Rgb(106, 205, 180); // Digital/tech accent
pub const ACCENT_BORDER: Color = Color::Rgb(160, 212, 190); // Companion frame

pub fn label() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub fn value() -> Style {
    Style::default().fg(TEXT)
}

pub fn prompt() -> Style {
    Style::default().fg(MINT).add_modifier(Modifier::DIM)
}

pub fn key() -> Style {
    Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
}

pub fn action() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub fn title_style() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

fn title_line(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(format!(" {title} "), title_style())]).alignment(Alignment::Left)
}

/// Quiet secondary frame; panels should feel like instruments, not dashboards.
pub fn panel_block(title: impl Into<String>) -> Block<'static> {
    let t = title.into();
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .title_top(title_line(&t))
        .title_alignment(Alignment::Left)
        .padding(Padding::new(1, 1, 0, 0))
}

pub fn alert_panel(title: impl Into<String>, critical: bool) -> Block<'static> {
    let accent = if critical { ERR } else { WARN };
    let t = title.into();
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent))
        .title_top(title_line(&t))
        .title_alignment(Alignment::Left)
        .padding(Padding::new(1, 1, 0, 0))
}

/// Single-line metric: `CPU  ████░░░░  21%`
pub fn metric_line(name: &str, percent: f64, fill: Color, width: u16) -> Line<'static> {
    let pct = percent.clamp(0.0, 100.0);
    let label_w = 5usize;
    let pct_w = 5usize;
    let bar_w = width.saturating_sub((label_w + pct_w) as u16).max(4) as usize;
    let filled = ((bar_w as f64) * pct / 100.0).round() as usize;
    let empty = bar_w.saturating_sub(filled);
    Line::from(vec![
        Span::styled(format!("{name:<label_w$} "), label()),
        Span::styled("█".repeat(filled), Style::default().fg(fill)),
        Span::styled("░".repeat(empty), Style::default().fg(BAR_EMPTY)),
        Span::styled(format!(" {:>3.0}%", pct), label()),
    ])
}

/// Thin horizontal telemetry row.
pub fn telemetry_row(name: &str, value: u64, max: u64, width: u16, color: Color) -> Line<'static> {
    let ratio = if max == 0 {
        0.0
    } else {
        value as f64 / max as f64
    };
    let label_show = if name.len() > 10 {
        format!("{}…", &name[..9])
    } else {
        format!("{name:<10}")
    };
    let bar_w = width.saturating_sub(14).max(4) as usize;
    let filled = (bar_w as f64 * ratio).round() as usize;
    let empty = bar_w.saturating_sub(filled);
    Line::from(vec![
        Span::styled(label_show, label()),
        Span::raw(" "),
        Span::styled("▏".repeat(filled), Style::default().fg(color)),
        Span::styled("░".repeat(empty), Style::default().fg(BAR_EMPTY)),
        Span::styled(format!(" {value:>3}"), label()),
    ])
}

pub fn pet_hotkeys_line() -> Line<'static> {
    Line::from(vec![
        Span::styled("F1", key()),
        Span::styled(" feed  ", action()),
        Span::styled("F2", key()),
        Span::styled(" play  ", action()),
        Span::styled("F3", key()),
        Span::styled(" vitals  ", action()),
        Span::styled("F4", key()),
        Span::styled(" name  ", action()),
        Span::styled("F5", key()),
        Span::styled(" scripts", action()),
    ])
    .alignment(Alignment::Center)
}

pub fn pet_vitals_line(hunger: u8, happiness: u8) -> Line<'static> {
    Line::from(vec![
        Span::styled("◇ hunger ", label()),
        Span::styled(format!("{hunger}"), Style::default().fg(ORANGE)),
        Span::styled("  ──  joy ", label()),
        Span::styled(format!("{happiness}"), Style::default().fg(CYAN)),
        Span::styled(" ◇", label()),
    ])
    .alignment(Alignment::Center)
}

/// Companion panel with custom border (digital aesthetic)
pub fn companion_panel_block(title: impl Into<String>) -> Block<'static> {
    let t = title.into();
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(ACCENT_BORDER))
        .title_top(title_line(&t))
        .title_alignment(Alignment::Left)
        .padding(Padding::new(1, 1, 0, 0))
}

/// System panel with tech aesthetic
pub fn system_panel_block(title: impl Into<String>) -> Block<'static> {
    let t = title.into();
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(BORDER))
        .title_top(title_line(&t))
        .title_alignment(Alignment::Left)
        .padding(Padding::new(1, 1, 0, 0))
}
