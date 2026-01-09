//! Terminal UI components for the gity demo.

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use std::time::Duration;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI mode.
pub fn init_terminal() -> io::Result<Term> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore terminal to normal mode.
pub fn restore_terminal(terminal: &mut Term) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Check if a key was pressed (non-blocking).
pub fn check_for_quit() -> io::Result<bool> {
    if event::poll(Duration::from_millis(10))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Wait for any key press.
pub fn wait_for_key() -> io::Result<()> {
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                return Ok(());
            }
        }
    }
}

/// Demo state for tracking progress.
#[derive(Default)]
pub struct DemoState {
    pub phase: usize,
    pub phase_name: String,
    pub phase_description: String,
    pub gity_time_ms: f64,
    pub baseline_time_ms: f64,
    pub file_count: usize,
    pub status_message: String,
    pub show_race: bool,
    pub race_progress_gity: f64,
    pub race_progress_baseline: f64,
    pub results: Vec<PhaseResult>,
}

#[derive(Clone)]
pub struct PhaseResult {
    pub name: String,
    pub gity_ms: f64,
    pub baseline_ms: f64,
}

impl PhaseResult {
    pub fn speedup(&self) -> f64 {
        if self.gity_ms > 0.0 {
            self.baseline_ms / self.gity_ms
        } else {
            0.0
        }
    }

    pub fn time_saved_ms(&self) -> f64 {
        self.baseline_ms - self.gity_ms
    }
}

/// Render the main demo UI.
pub fn render_demo(frame: &mut Frame, state: &DemoState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),  // Title
            Constraint::Length(3),  // Phase info
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Status bar
        ])
        .split(frame.area());

    render_title(frame, chunks[0]);
    render_phase_info(frame, chunks[1], state);

    if state.show_race {
        render_race(frame, chunks[2], state);
    } else if !state.results.is_empty() && state.phase == 0 {
        render_summary(frame, chunks[2], state);
    } else {
        render_comparison(frame, chunks[2], state);
    }

    render_status_bar(frame, chunks[3], state);
}

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(vec![Line::from(vec![
        Span::styled("GITY ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled("PERFORMANCE DEMO", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));
    frame.render_widget(title, area);
}

fn render_phase_info(frame: &mut Frame, area: Rect, state: &DemoState) {
    let phase_text = if state.phase > 0 {
        format!("Act {}: {} - {}", state.phase, state.phase_name, state.phase_description)
    } else {
        state.phase_description.clone()
    };

    let info = Paragraph::new(phase_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(info, area);
}

fn render_comparison(frame: &mut Frame, area: Rect, state: &DemoState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // With gity panel
    let gity_block = Block::default()
        .title(" WITH GITY ")
        .title_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let gity_content = if state.gity_time_ms > 0.0 {
        format!("\n\n    {:.1}ms", state.gity_time_ms)
    } else {
        "\n\n    Waiting...".to_string()
    };

    let gity_para = Paragraph::new(gity_content)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(gity_block);
    frame.render_widget(gity_para, chunks[0]);

    // Without gity panel
    let baseline_block = Block::default()
        .title(" WITHOUT GITY ")
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let baseline_content = if state.baseline_time_ms > 0.0 {
        format!("\n\n    {:.1}ms", state.baseline_time_ms)
    } else {
        "\n\n    Waiting...".to_string()
    };

    let baseline_para = Paragraph::new(baseline_content)
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(baseline_block);
    frame.render_widget(baseline_para, chunks[1]);
}

fn render_race(frame: &mut Frame, area: Rect, state: &DemoState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    let block = Block::default()
        .title(" GIT STATUS RACE ")
        .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL);
    frame.render_widget(block, area);

    // Gity progress
    let gity_label = if state.race_progress_gity >= 1.0 {
        format!("WITH GITY:     DONE! {:.0}ms", state.gity_time_ms)
    } else {
        format!("WITH GITY:     {:.0}%", state.race_progress_gity * 100.0)
    };
    let gity_gauge = Gauge::default()
        .label(gity_label)
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(state.race_progress_gity.min(1.0));
    frame.render_widget(gity_gauge, chunks[1]);

    // Baseline progress
    let baseline_label = if state.race_progress_baseline >= 1.0 {
        format!("WITHOUT GITY:  DONE! {:.0}ms", state.baseline_time_ms)
    } else {
        format!("WITHOUT GITY:  {:.0}%", state.race_progress_baseline * 100.0)
    };
    let baseline_gauge = Gauge::default()
        .label(baseline_label)
        .gauge_style(Style::default().fg(Color::Red))
        .ratio(state.race_progress_baseline.min(1.0));
    frame.render_widget(baseline_gauge, chunks[3]);

    // Winner announcement
    if state.race_progress_gity >= 1.0 && state.race_progress_baseline >= 1.0 {
        let speedup = state.baseline_time_ms / state.gity_time_ms.max(0.001);
        let saved = state.baseline_time_ms - state.gity_time_ms;
        let winner_text = format!("gity wins by {:.0}ms ({:.1}x faster)", saved, speedup);
        let winner = Paragraph::new(winner_text)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        frame.render_widget(winner, chunks[4]);
    }
}

fn render_summary(frame: &mut Frame, area: Rect, state: &DemoState) {
    let block = Block::default()
        .title(" DEMO COMPLETE ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),  // File count
            Constraint::Min(6),     // Results table
            Constraint::Length(3),  // Total saved
            Constraint::Length(2),  // CTA
        ])
        .split(inner);

    // File count
    let file_info = Paragraph::new(format!("Repository: {} files", state.file_count))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    frame.render_widget(file_info, chunks[0]);

    // Results table
    let header = Row::new(vec!["Operation", "Speedup", "Time Saved"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(1);

    let rows: Vec<Row> = state.results.iter().map(|r| {
        let speedup_color = if r.speedup() > 5.0 { Color::Green } else { Color::Yellow };
        Row::new(vec![
            r.name.clone(),
            format!("{:.1}x", r.speedup()),
            format!("{:.0}ms", r.time_saved_ms()),
        ])
        .style(Style::default().fg(speedup_color))
    }).collect();

    let table = Table::new(rows, [
        Constraint::Length(20),
        Constraint::Length(12),
        Constraint::Length(12),
    ])
    .header(header)
    .block(Block::default());
    frame.render_widget(table, chunks[1]);

    // Total time saved
    let total_saved: f64 = state.results.iter().map(|r| r.time_saved_ms()).sum();
    let daily_estimate = (total_saved * 100.0) / 1000.0 / 60.0; // Assume 100 ops/day
    let total_text = format!(
        "Estimated time saved per day: {:.0} minutes",
        daily_estimate.max(5.0)
    );
    let total = Paragraph::new(total_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    frame.render_widget(total, chunks[2]);

    // CTA
    let cta = Paragraph::new("Get started: cargo install gity  |  https://github.com/neul-labs/gity")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(cta, chunks[3]);
}

fn render_status_bar(frame: &mut Frame, area: Rect, state: &DemoState) {
    let speedup = if state.gity_time_ms > 0.0 && state.baseline_time_ms > 0.0 {
        let s = state.baseline_time_ms / state.gity_time_ms;
        format!(" | Speedup: {:.1}x", s)
    } else {
        String::new()
    };

    let status = format!(
        "{}{} | Press 'q' to quit",
        state.status_message,
        speedup
    );

    let status_bar = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(status_bar, area);
}
