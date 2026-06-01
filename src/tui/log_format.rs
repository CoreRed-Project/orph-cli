use crate::tui::theme;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// Build a styled log line with semantic level coloring and trimmed body.
pub fn log_line_spans(raw: &str, max_width: usize) -> Line<'static> {
    let (level, body) = parse_level_body(raw);
    let body = shorten_body(&body, max_width.saturating_sub(8));

    let level_span = match level {
        LogLevel::Error => Span::styled("ERR ", Style::default().fg(theme::ERR)),
        LogLevel::Warn => Span::styled("WRN ", Style::default().fg(theme::WARN)),
        LogLevel::Info => Span::styled("INF ", Style::default().fg(theme::TEXT_DIM)),
        LogLevel::Plain => Span::raw(""),
    };

    Line::from(vec![
        level_span,
        Span::styled(body, Style::default().fg(theme::TEXT)),
    ])
}

#[derive(Clone, Copy)]
enum LogLevel {
    Info,
    Warn,
    Error,
    Plain,
}

fn parse_level_body(line: &str) -> (LogLevel, String) {
    if let Some(rest) = line.strip_prefix("ERROR:") {
        return (LogLevel::Error, rest.trim().to_string());
    }
    if let Some(rest) = line.strip_prefix("WARN:") {
        return (LogLevel::Warn, rest.trim().to_string());
    }
    if let Some(rest) = line.strip_prefix("INFO:") {
        return (LogLevel::Info, rest.trim().to_string());
    }
    if line.contains("[ERROR]") {
        return (LogLevel::Error, extract_after_bracket(line));
    }
    if line.contains("[WARN]") {
        return (LogLevel::Warn, extract_after_bracket(line));
    }
    if line.contains("[INFO]") {
        return (LogLevel::Info, extract_after_bracket(line));
    }
    (LogLevel::Plain, line.to_string())
}

fn extract_after_bracket(line: &str) -> String {
    line.find(']')
        .map(|i| line[i + 1..].trim().to_string())
        .unwrap_or_else(|| line.to_string())
}

fn shorten_body(body: &str, max: usize) -> String {
    let compact = if body.contains("pet decay applied") {
        if let Some(elapsed) = body.split("elapsed=").nth(1) {
            let hrs = elapsed.split('h').next().unwrap_or("?").trim();
            format!("pet decay · {hrs}h")
        } else {
            "pet decay".into()
        }
    } else if let Some(cmd) = body.strip_prefix("command:") {
        format!("cmd {}", cmd.trim())
    } else if body.starts_with("running script:") {
        body.replacen("running script:", "run", 1)
    } else if body.starts_with("script '") {
        body.split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        body.to_string()
    };

    if compact.len() <= max {
        compact
    } else if max > 1 {
        format!("{}…", &compact[..max.saturating_sub(1)])
    } else {
        compact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortens_decay_noise() {
        let s = shorten_body(
            "pet decay applied: elapsed=1.23h hunger 30→35 happiness 70→65",
            40,
        );
        assert!(s.contains("pet decay"));
        assert!(!s.contains("happiness"));
    }
}
