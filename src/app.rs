use chrono::{Duration as ChronoDuration, Local, NaiveTime, Timelike};
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use enigo::{Coordinate, Enigo, Mouse, Settings};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use std::{
    io::{self, stdout},
    thread,
    time::{Duration, Instant},
};

pub struct App {
    stop_time: NaiveTime,
    last_mouse_move: Instant,
    verbose: bool,
}

impl App {
    pub fn new() -> Self {
        // Default shutdown time: 18:00 (6:00 PM)
        let mut target_time = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
        let now = Local::now();
        let current_time = now.time();

        // If we've already passed the stop time, add 30 minutes from now
        if current_time >= target_time {
            let minutes_from_now = current_time.hour() * 60 + current_time.minute() + 30;
            let new_hour = (minutes_from_now / 60) % 24;
            let new_minute = minutes_from_now % 60;
            target_time = NaiveTime::from_hms_opt(new_hour, new_minute, 0).unwrap();
        }

        Self {
            stop_time: target_time,
            last_mouse_move: Instant::now(),
            verbose: false, // Start in discrete mode
        }
    }

    pub fn toggle_verbose(&mut self) {
        self.verbose = !self.verbose;
    }

    pub fn adjust_time(&mut self, minutes: i32) {
        if let Some(duration) = ChronoDuration::try_minutes(i64::from(minutes)) {
            // Add duration and wrap around 24 hours
            let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            let seconds_since_midnight = self.stop_time.signed_duration_since(midnight).num_seconds();
            let new_seconds = (seconds_since_midnight + duration.num_seconds()).rem_euclid(86400);

            if let Some(new_time) = NaiveTime::from_num_seconds_from_midnight_opt(new_seconds as u32, 0) {
                self.stop_time = new_time;
            }
        }
    }

    pub fn should_stop(&self) -> bool {
        let now = Local::now();
        let current_time = now.time();

        // Check if current time matches target time (within the same minute)
        current_time.hour() == self.stop_time.hour() && current_time.minute() == self.stop_time.minute()
    }

    pub fn time_remaining(&self) -> Option<ChronoDuration> {
        let now = Local::now();
        let current_time = now.time();
        let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

        let current_seconds = current_time.signed_duration_since(midnight).num_seconds();
        let target_seconds = self.stop_time.signed_duration_since(midnight).num_seconds();

        let remaining_seconds = if target_seconds >= current_seconds {
            target_seconds - current_seconds
        } else {
            86400 - current_seconds + target_seconds
        };

        ChronoDuration::try_seconds(remaining_seconds)
    }

    pub fn move_mouse_if_needed(&mut self) {
        if self.last_mouse_move.elapsed() >= Duration::from_secs(300) {
            if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
                let _ = enigo.move_mouse(10, 10, Coordinate::Rel);
                thread::sleep(Duration::from_millis(100));
                let _ = enigo.move_mouse(-10, -10, Coordinate::Rel);
            }
            self.last_mouse_move = Instant::now();
        }
    }
}

pub fn run() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout()))?;

    let mut app = App::new();

    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_app(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Handle events with timeout
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match handle_key_event(key, app) {
                        KeyAction::Quit => break,
                        KeyAction::Continue => {}
                    }
                }
            }
        }

        // Check if it's time to stop
        if app.should_stop() {
            break;
        }

        // Move mouse periodically
        app.move_mouse_if_needed();
    }

    Ok(())
}

enum KeyAction {
    Quit,
    Continue,
}

fn handle_key_event(key: KeyEvent, app: &mut App) -> KeyAction {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => KeyAction::Quit,
        KeyCode::Char('v') | KeyCode::Char('V') => {
            app.toggle_verbose();
            KeyAction::Continue
        }
        KeyCode::Char('+') | KeyCode::Char('=') => {
            app.adjust_time(30);
            KeyAction::Continue
        }
        KeyCode::Char('-') | KeyCode::Char('_') => {
            app.adjust_time(-30);
            KeyAction::Continue
        }
        _ => KeyAction::Continue,
    }
}

fn ui(f: &mut Frame, app: &App) {
    if app.verbose {
        ui_verbose(f, app);
    } else {
        ui_discrete(f, app);
    }
}

fn ui_discrete(f: &mut Frame, app: &App) {
    let now = Local::now();

    // Minimal layout - just time and status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Current time
            Constraint::Min(1),    // Empty space
            Constraint::Length(1), // Status
        ])
        .split(f.area());

    // Current Time (minimal)
    let current_time_widget = Paragraph::new(now.format("%H:%M:%S").to_string())
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(current_time_widget, chunks[0]);

    // Status (minimal)
    let status = if app.should_stop() {
        Span::styled("■", Style::default().fg(Color::Red))
    } else {
        Span::styled("●", Style::default().fg(Color::DarkGray))
    };

    let status_widget = Paragraph::new(Line::from(vec![status, Span::raw(" v: details")]))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(status_widget, chunks[2]);
}

fn ui_verbose(f: &mut Frame, app: &App) {
    let now = Local::now();

    let (minutes_remaining, hours_remaining, mins_remaining) =
        if let Some(remaining) = app.time_remaining() {
            let total_minutes = remaining.num_minutes();
            let hours_remaining = total_minutes / 60;
            let mins_remaining = total_minutes % 60;

            (total_minutes, hours_remaining, mins_remaining)
        } else {
            // Fallback if calculation fails
            (0, 0, 0)
        };

    // Calculate progress (0.0 to 1.0)
    let progress = if minutes_remaining > 0 {
        1.0 - (minutes_remaining as f64 / 1440.0)
    } else {
        1.0
    };

    // Main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Current time
            Constraint::Length(3), // Stop time
            Constraint::Length(3), // Time remaining
            Constraint::Length(3), // Progress bar
            Constraint::Min(5),    // Instructions
            Constraint::Length(3), // Status
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("Slippery Bastard")
        .style(Style::default().fg(Color::Cyan).bold())
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(title, chunks[0]);

    // Current Time
    let current_time_widget = Paragraph::new(vec![Line::from(vec![
        Span::raw("Current Time: "),
        Span::styled(
            now.format("%H:%M:%S").to_string(),
            Style::default().fg(Color::Green).bold(),
        ),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(current_time_widget, chunks[1]);

    // Stop Time
    let stop_time_widget = Paragraph::new(vec![Line::from(vec![
        Span::raw("Stop Time: "),
        Span::styled(
            app.stop_time.format("%H:%M").to_string(),
            Style::default().fg(Color::Yellow).bold(),
        ),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(stop_time_widget, chunks[2]);

    // Time Remaining
    let time_color = if minutes_remaining <= 30 {
        Color::Red
    } else if minutes_remaining <= 60 {
        Color::Yellow
    } else {
        Color::Green
    };

    let time_remaining_widget = Paragraph::new(vec![Line::from(vec![
        Span::raw("Time Remaining: "),
        Span::styled(
            format!("{}h {:02}m", hours_remaining, mins_remaining),
            Style::default().fg(time_color).bold(),
        ),
    ])])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(time_remaining_widget, chunks[3]);

    // Progress bar
    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(
            Style::default()
                .fg(if progress >= 0.9 {
                    Color::Red
                } else if progress >= 0.75 {
                    Color::Yellow
                } else {
                    Color::Green
                })
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .ratio(progress);
    f.render_widget(gauge, chunks[4]);

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![Span::styled(
            "Keyboard Controls:",
            Style::default().fg(Color::Cyan).bold(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  +/=  ", Style::default().fg(Color::Green)),
            Span::raw("Add 30 minutes to stop time"),
        ]),
        Line::from(vec![
            Span::styled("  -/_  ", Style::default().fg(Color::Red)),
            Span::raw("Subtract 30 minutes from stop time"),
        ]),
        Line::from(vec![
            Span::styled("  v    ", Style::default().fg(Color::Cyan)),
            Span::raw(" Toggle discrete/verbose mode"),
        ]),
        Line::from(vec![
            Span::styled("  q/ESC", Style::default().fg(Color::Yellow)),
            Span::raw(" Quit immediately"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Note: ", Style::default().fg(Color::Magenta).italic()),
            Span::raw("Mouse moves every 5 minutes to prevent screen lock"),
        ]),
    ])
    .alignment(Alignment::Left)
    .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(instructions, chunks[5]);

    // Status
    let status = if app.should_stop() {
        Span::styled("■ STOPPING", Style::default().fg(Color::Red).bold())
    } else {
        Span::styled("● RUNNING", Style::default().fg(Color::Green).bold())
    };

    let status_widget = Paragraph::new(Line::from(vec![status]))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status_widget, chunks[6]);
}
