use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
    Terminal,
};
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Deserialize, Default, Clone)]
struct TelemetryPayload {
    gpu_thermal_celsius: f32,
    inference_latency_ms: f32,
    active_tokens: u32,
    event_epoch_seq: u64,
    status: String,
}

struct App {
    telemetry: Arc<Mutex<TelemetryPayload>>,
    connected: Arc<Mutex<bool>>,
}

impl App {
    fn new() -> App {
        App {
            telemetry: Arc::new(Mutex::new(TelemetryPayload::default())),
            connected: Arc::new(Mutex::new(false)),
        }
    }
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();

    // Spawn background thread to connect to Unix socket and update telemetry
    let telemetry_clone = Arc::clone(&app.telemetry);
    let connected_clone = Arc::clone(&app.connected);
    std::thread::spawn(move || {
        let socket_path = "/tmp/tesseract_shader.sock";
        loop {
            match UnixStream::connect(socket_path) {
                Ok(stream) => {
                    *connected_clone.lock().unwrap() = true;
                    let reader = BufReader::new(stream);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if let Ok(payload) = serde_json::from_str::<TelemetryPayload>(&line) {
                                *telemetry_clone.lock().unwrap() = payload;
                            }
                        } else {
                            break;
                        }
                    }
                    *connected_clone.lock().unwrap() = false;
                }
                Err(_) => {
                    *connected_clone.lock().unwrap() = false;
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
        }
    });

    // Run app loop
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(5),
            ]
            .as_ref(),
        )
        .split(f.size());

    let connected = *app.connected.lock().unwrap();
    let telemetry = app.telemetry.lock().unwrap().clone();

    // Header
    let header_text = if connected {
        format!(
            " Tesseract OS Holographic Diagnostic Matrix // {} ",
            telemetry.status
        )
    } else {
        " Tesseract OS Holographic Diagnostic Matrix // [DISCONNECTED] ".to_string()
    };

    let header_color = if connected {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(header_color)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL).title("Core"));
    f.render_widget(header, chunks[0]);

    // GPU Thermal Gauge
    let temp = telemetry.gpu_thermal_celsius;
    // Map 0-100C to 0.0-1.0 ratio
    let ratio = (temp / 100.0).clamp(0.0, 1.0) as f64;
    let temp_color = if temp > 85.0 {
        Color::Red
    } else if temp > 65.0 {
        Color::Yellow
    } else {
        Color::Green
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title("GPU Thermal Load (Carnot)")
                .borders(Borders::ALL),
        )
        .gauge_style(
            Style::default()
                .fg(temp_color)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )
        .ratio(ratio)
        .label(format!("{:.1}°C", temp));
    f.render_widget(gauge, chunks[1]);

    // Metrics Box
    let metrics_text = format!(
        "\n  Inference Latency:   {:.2} ms\n  Active Tokens:       {}\n  Event Epoch Seq:     {}\n\n  Press 'q' to quit.",
        telemetry.inference_latency_ms, telemetry.active_tokens, telemetry.event_epoch_seq
    );

    let metrics_box = Paragraph::new(metrics_text)
        .style(Style::default().fg(Color::Magenta))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cognitive Immune System"),
        );
    f.render_widget(metrics_box, chunks[2]);
}
