use ratatui::layout::Alignment;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::block::Padding;
use ratatui::widgets::{Block, BorderType, Borders};

// ── Fruity & Cute Pastel Aesthetic: Soft machine vibes by Sxnnyside ───────

pub const STRAWBERRY: Color = Color::Rgb(255, 145, 150); // Soft pastel pink-red
pub const PEACH: Color = Color::Rgb(255, 185, 150); // Warm pastel orange
pub const BANANA: Color = Color::Rgb(250, 230, 160); // Soft pastel yellow
pub const MINT: Color = Color::Rgb(160, 235, 195); // Soft leafy mint
pub const BLUEBERRY: Color = Color::Rgb(155, 210, 255); // Soft sky/blueberry
pub const LAVENDER: Color = Color::Rgb(215, 185, 255); // Soft pastel lavender

pub const ORANGE: Color = PEACH;
pub const CYAN: Color = BLUEBERRY;
pub const PET_ART: Color = Color::Rgb(245, 230, 235); // Soft cream/blush
pub const TEXT: Color = Color::Rgb(235, 235, 235); // Readable white
pub const TEXT_DIM: Color = Color::Rgb(145, 165, 175); // Soft pastel slate
pub const BORDER: Color = Color::Rgb(90, 115, 125); // Gentle frames
pub const BAR_EMPTY: Color = Color::Rgb(40, 50, 55); // Soft dark gap
pub const WARN: Color = BANANA;
pub const ERR: Color = STRAWBERRY;
pub const GLOW_WARM: Color = STRAWBERRY;
pub const ACCENT_BORDER: Color = LAVENDER;

pub fn label() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub fn value() -> Style {
    Style::default().fg(TEXT)
}

pub fn prompt() -> Style {
    Style::default().fg(MINT).add_modifier(Modifier::BOLD)
}

pub fn key() -> Style {
    Style::default().fg(PEACH).add_modifier(Modifier::BOLD)
}

pub fn action() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub fn title_style() -> Style {
    Style::default().fg(BLUEBERRY).add_modifier(Modifier::BOLD)
}

fn title_line(title: &str) -> Line<'static> {
    Line::from(vec![Span::styled(format!(" {title} "), title_style())]).alignment(Alignment::Left)
}

/// Quiet secondary frame; panels should feel like instruments, not dashboards.
pub fn panel_block(title: impl Into<String>, focused: bool) -> Block<'static> {
    let t = title.into();
    let border_style = if focused {
        Style::default().fg(MINT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(BORDER)
    };
    let border_type = if focused {
        BorderType::Thick
    } else {
        BorderType::Rounded
    };
    let display_title = if focused { format!("▶ {}", t) } else { t };

    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .title_top(title_line(&display_title))
        .title_alignment(Alignment::Left)
        .padding(Padding::new(1, 1, 0, 0))
}

pub fn alert_panel(title: impl Into<String>, critical: bool, focused: bool) -> Block<'static> {
    let accent = if critical { ERR } else { WARN };
    let border_style = if focused {
        Style::default().fg(accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(accent)
    };
    let border_type = if focused {
        BorderType::Thick
    } else {
        BorderType::Double
    };
    let t = title.into();
    let display_title = if focused { format!("▶ {}", t) } else { t };

    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .title_top(title_line(&display_title))
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
pub fn companion_panel_block(title: impl Into<String>, focused: bool) -> Block<'static> {
    let t = title.into();
    let border_style = if focused {
        Style::default().fg(MINT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ACCENT_BORDER)
    };
    let border_type = if focused {
        BorderType::Thick
    } else {
        BorderType::Double
    };
    let display_title = if focused { format!("▶ {}", t) } else { t };

    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .title_top(title_line(&display_title))
        .title_alignment(Alignment::Left)
        .padding(Padding::new(1, 1, 0, 0))
}

/// System panel with tech aesthetic
pub fn system_panel_block(title: impl Into<String>, focused: bool) -> Block<'static> {
    let t = title.into();
    let border_style = if focused {
        Style::default().fg(MINT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(BORDER)
    };
    let border_type = if focused {
        BorderType::Thick
    } else {
        BorderType::Plain
    };
    let display_title = if focused { format!("▶ {}", t) } else { t };

    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(border_style)
        .title_top(title_line(&display_title))
        .title_alignment(Alignment::Left)
        .padding(Padding::new(1, 1, 0, 0))
}
