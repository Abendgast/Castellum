use std::{
    f64::consts::PI,
    io,
    time::{Duration, Instant},
};

use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::Rng;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Circle, Line as CanvasLine, Points},
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Row,
        Sparkline, Table, TableState, Tabs, Wrap,
    },
    Frame, Terminal,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ActiveTab {
    KineticLab = 0,
    SecurityTelemetry = 1,
    DataMatrix = 2,
    Scratchpad = 3,
}

impl ActiveTab {
    fn from_index(i: usize) -> Self {
        match i % 4 {
            0 => ActiveTab::KineticLab,
            1 => ActiveTab::SecurityTelemetry,
            2 => ActiveTab::DataMatrix,
            _ => ActiveTab::Scratchpad,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KineticMode {
    Spinner,
    Cube3D,
    Lissajous,
}

struct Particle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    color: Color,
}

struct AppState {
    tab: ActiveTab,
    tick_count: u64,
    start_time: Instant,
    last_tick: Instant,
    show_help: bool,

    // Tab 1: Kinetic Fidget
    kinetic_mode: KineticMode,
    angle: f64,
    angular_velocity: f64,
    spinner_arms: usize,
    bubble_grid: [[bool; 4]; 4],
    bubble_cursor: (usize, usize),
    bubbles_popped: u64,
    pop_feedback: String,
    particles: Vec<Particle>,
    gravity_enabled: bool,
    stress_level: u16,

    // Tab 2: Security Telemetry
    entropy_history: Vec<u64>,
    current_entropy: f64,
    high_entropy_stream: bool,
    memory_blocks: Vec<(&'static str, Color)>,
    panic_wipe_anim: u8,
    argon_progress: u16,
    totp_countdown: u8,
    totp_code: u32,

    // Tab 3: Data Matrix
    table_state: TableState,
    log_messages: Vec<(String, &'static str, Color)>,
    log_autoscroll: bool,

    // Tab 4: Scratchpad & Generator
    pass_length: usize,
    pass_upper: bool,
    pass_digits: bool,
    pass_symbols: bool,
    pass_diceware: bool,
    generated_pass: String,
    scratchpad_text: String,
    _scratchpad_cursor: usize,
}

impl AppState {
    fn new() -> Self {
        let mut rng = rand::thread_rng();

        // Initialize particles
        let colors = [
            Color::Cyan,
            Color::Green,
            Color::Yellow,
            Color::Magenta,
            Color::LightBlue,
            Color::LightGreen,
        ];
        let mut particles = Vec::new();
        for _ in 0..12 {
            particles.push(Particle {
                x: rng.gen_range(-30.0..30.0),
                y: rng.gen_range(-30.0..30.0),
                vx: rng.gen_range(-1.2..1.2),
                vy: rng.gen_range(-1.2..1.2),
                color: colors[rng.gen_range(0..colors.len())],
            });
        }

        // Initialize memory blocks simulation (64 blocks)
        let block_types = [
            ("0x00", Color::DarkGray),
            ("SECR", Color::Cyan),
            ("LOCK", Color::Green),
            ("ZERO", Color::Blue),
            ("RAND", Color::Magenta),
        ];
        let mut memory_blocks = Vec::new();
        for _ in 0..64 {
            let idx = rng.gen_range(0..block_types.len());
            memory_blocks.push(block_types[idx]);
        }

        let mut app = Self {
            tab: ActiveTab::KineticLab,
            tick_count: 0,
            start_time: Instant::now(),
            last_tick: Instant::now(),
            show_help: false,

            kinetic_mode: KineticMode::Spinner,
            angle: 0.0,
            angular_velocity: 8.0,
            spinner_arms: 4,
            bubble_grid: [[false; 4]; 4],
            bubble_cursor: (0, 0),
            bubbles_popped: 0,
            pop_feedback: String::from("Ready to pop!"),
            particles,
            gravity_enabled: false,
            stress_level: 25,

            entropy_history: vec![790, 792, 795, 798, 799, 794, 799, 800, 797, 799, 800, 798, 799],
            current_entropy: 7.994,
            high_entropy_stream: true,
            memory_blocks,
            panic_wipe_anim: 0,
            argon_progress: 42,
            totp_countdown: 30,
            totp_code: 849201,

            table_state: TableState::default().with_selected(Some(0)),
            log_messages: vec![
                (
                    Local::now().format("%H:%M:%S").to_string(),
                    "[BOOT] Castellum Security Workbench v0.1.0 initialized",
                    Color::Green,
                ),
                (
                    Local::now().format("%H:%M:%S").to_string(),
                    "[SYS] sys_memfd_secret syscall 447 verified (Active)",
                    Color::Cyan,
                ),
                (
                    Local::now().format("%H:%M:%S").to_string(),
                    "[SYS] mlockall(MCL_CURRENT | MCL_FUTURE) swap lock active",
                    Color::Green,
                ),
                (
                    Local::now().format("%H:%M:%S").to_string(),
                    "[SEC] prctl(PR_SET_DUMPABLE, 0) ptrace defense engaged",
                    Color::Yellow,
                ),
                (
                    Local::now().format("%H:%M:%S").to_string(),
                    "[SANDBOX] Landlock LSM ruleset applied. Socket calls dropped",
                    Color::LightCyan,
                ),
            ],
            log_autoscroll: true,

            pass_length: 24,
            pass_upper: true,
            pass_digits: true,
            pass_symbols: true,
            pass_diceware: false,
            generated_pass: String::new(),
            scratchpad_text: String::from("Castellum offline encrypted secure pad.\nZeroization verified on drop."),
            _scratchpad_cursor: 60,
        };

        app.regenerate_password();
        app
    }

    fn update(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Kinetic spinner physics
        self.angle += self.angular_velocity * 0.05;
        if self.angle > 2.0 * PI {
            self.angle -= 2.0 * PI;
        } else if self.angle < 0.0 {
            self.angle += 2.0 * PI;
        }
        // Inertia friction
        self.angular_velocity *= 0.992;
        if self.angular_velocity.abs() < 0.01 {
            self.angular_velocity = 0.0;
        }

        // Particle physics
        let bound = 45.0;
        for p in &mut self.particles {
            if self.gravity_enabled {
                p.vy -= 0.08;
            }
            p.x += p.vx;
            p.y += p.vy;

            if p.x.abs() > bound {
                p.vx = -p.vx * 0.98;
                p.x = p.x.signum() * bound;
            }
            if p.y.abs() > bound {
                p.vy = -p.vy * 0.98;
                p.y = p.y.signum() * bound;
            }
        }

        // Argon progress simulation
        if self.tick_count % 3 == 0 {
            self.argon_progress = (self.argon_progress + 1) % 101;
        }

        // TOTP countdown
        if self.tick_count % 30 == 0 {
            if self.totp_countdown == 0 {
                self.totp_countdown = 30;
                let mut rng = rand::thread_rng();
                self.totp_code = rng.gen_range(100_000..999_999);
                self.add_log(
                    format!("[TOTP] RFC 6238 token refreshed: {:06}", self.totp_code),
                    Color::Magenta,
                );
            } else {
                self.totp_countdown -= 1;
            }
        }

        // Entropy simulation stream
        if self.tick_count % 5 == 0 {
            let mut rng = rand::thread_rng();
            let val = if self.high_entropy_stream {
                rng.gen_range(795..=800)
            } else {
                rng.gen_range(410..=550)
            };
            self.entropy_history.push(val);
            if self.entropy_history.len() > 80 {
                self.entropy_history.remove(0);
            }
            self.current_entropy = val as f64 / 100.0;
        }

        // Panic wipe animation cascade
        if self.panic_wipe_anim > 0 {
            let idx = (64 - self.panic_wipe_anim as usize) % self.memory_blocks.len();
            self.memory_blocks[idx] = ("0x00", Color::Green);
            self.panic_wipe_anim -= 1;
        }
    }

    fn add_log(&mut self, text: String, color: Color) {
        let ts = Local::now().format("%H:%M:%S").to_string();
        self.log_messages.push((ts, Box::leak(text.into_boxed_str()), color));
        if self.log_messages.len() > 100 {
            self.log_messages.remove(0);
        }
    }

    fn pop_current_bubble(&mut self) {
        let (r, c) = self.bubble_cursor;
        self.bubble_grid[r][c] = !self.bubble_grid[r][c];
        self.bubbles_popped += 1;
        self.stress_level = self.stress_level.saturating_sub(2);

        let sounds = ["*SNAP!*", "*CRACK!*", "*CLICK!*", "*POP!*", "*BEEP!*"];
        let mut rng = rand::thread_rng();
        self.pop_feedback = format!(
            "{} Cell ({}, {}) Popped! Total: {}",
            sounds[rng.gen_range(0..sounds.len())],
            r + 1,
            c + 1,
            self.bubbles_popped
        );
    }

    fn trigger_panic_wipe(&mut self) {
        self.panic_wipe_anim = 64;
        for b in &mut self.memory_blocks {
            *b = ("WIPE", Color::Red);
        }
        self.add_log(
            String::from("[PANIC] Panic Wipe triggered! Wiping all memfd_secret pages..."),
            Color::Red,
        );
    }

    fn regenerate_password(&mut self) {
        let mut rng = rand::thread_rng();
        if self.pass_diceware {
            let words = [
                "correct", "horse", "battery", "staple", "cipher", "citadel", "fortress",
                "castellum", "enigma", "argon", "secret", "kernel", "quantum", "entropy",
                "shield", "matrix", "vector", "stream", "debian", "trixie",
            ];
            let mut picked = Vec::new();
            for _ in 0..4 {
                picked.push(words[rng.gen_range(0..words.len())]);
            }
            self.generated_pass = picked.join("-");
        } else {
            let mut charset = Vec::new();
            charset.extend(b'a'..=b'z');
            if self.pass_upper {
                charset.extend(b'A'..=b'Z');
            }
            if self.pass_digits {
                charset.extend(b'0'..=b'9');
            }
            if self.pass_symbols {
                charset.extend(b"!@#$%^&*()-_=+[]{}|;:,.<>?");
            }
            if charset.is_empty() {
                charset.extend(b'a'..=b'z');
            }

            let pass: String = (0..self.pass_length)
                .map(|_| {
                    let idx = rng.gen_range(0..charset.len());
                    charset[idx] as char
                })
                .collect();
            self.generated_pass = pass;
        }
    }
}

fn main() -> Result<(), io::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = AppState::new();
    let tick_rate = Duration::from_millis(20); // ~50 FPS for smooth kinetic animation

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate.saturating_sub(app.last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Global keys
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char('?') | KeyCode::Char('h') => {
                            app.show_help = !app.show_help;
                        }
                        KeyCode::Tab => {
                            app.tab = ActiveTab::from_index(app.tab as usize + 1);
                        }
                        KeyCode::BackTab => {
                            app.tab = ActiveTab::from_index((app.tab as usize + 3) % 4);
                        }
                        KeyCode::Char('1') => app.tab = ActiveTab::KineticLab,
                        KeyCode::Char('2') => app.tab = ActiveTab::SecurityTelemetry,
                        KeyCode::Char('3') => app.tab = ActiveTab::DataMatrix,
                        KeyCode::Char('4') => app.tab = ActiveTab::Scratchpad,

                        // Tab-specific interactions
                        _ => match app.tab {
                            ActiveTab::KineticLab => match key.code {
                                KeyCode::Char(' ') | KeyCode::Char('+') => {
                                    app.angular_velocity += 15.0;
                                    app.stress_level = app.stress_level.saturating_add(5).min(100);
                                }
                                KeyCode::Char('-') => {
                                    app.angular_velocity *= 0.5;
                                }
                                KeyCode::Char('r') => {
                                    app.angular_velocity = -app.angular_velocity;
                                }
                                KeyCode::Char('m') => {
                                    app.kinetic_mode = match app.kinetic_mode {
                                        KineticMode::Spinner => KineticMode::Cube3D,
                                        KineticMode::Cube3D => KineticMode::Lissajous,
                                        KineticMode::Lissajous => KineticMode::Spinner,
                                    };
                                }
                                KeyCode::Char('a') => {
                                    app.spinner_arms = match app.spinner_arms {
                                        3 => 4,
                                        4 => 6,
                                        6 => 8,
                                        _ => 3,
                                    };
                                }
                                KeyCode::Char('g') => {
                                    app.gravity_enabled = !app.gravity_enabled;
                                    app.add_log(
                                        format!("[PHYS] Gravity toggle: {}", app.gravity_enabled),
                                        Color::Cyan,
                                    );
                                }
                                KeyCode::Up => {
                                    app.bubble_cursor.0 = (app.bubble_cursor.0 + 3) % 4;
                                }
                                KeyCode::Down => {
                                    app.bubble_cursor.0 = (app.bubble_cursor.0 + 1) % 4;
                                }
                                KeyCode::Left => {
                                    app.bubble_cursor.1 = (app.bubble_cursor.1 + 3) % 4;
                                }
                                KeyCode::Right => {
                                    app.bubble_cursor.1 = (app.bubble_cursor.1 + 1) % 4;
                                }
                                KeyCode::Enter => {
                                    app.pop_current_bubble();
                                }
                                _ => {}
                            },
                            ActiveTab::SecurityTelemetry => match key.code {
                                KeyCode::Char('w') => app.trigger_panic_wipe(),
                                KeyCode::Char('e') => {
                                    app.high_entropy_stream = !app.high_entropy_stream;
                                    app.add_log(
                                        format!(
                                            "[ENTROPY] Mode toggled to {}",
                                            if app.high_entropy_stream {
                                                "CSPRNG (H >= 7.99)"
                                            } else {
                                                "LOW (Plaintext)"
                                            }
                                        ),
                                        Color::Yellow,
                                    );
                                }
                                _ => {}
                            },
                            ActiveTab::DataMatrix => match key.code {
                                KeyCode::Up => {
                                    let i = match app.table_state.selected() {
                                        Some(i) => {
                                            if i == 0 {
                                                4
                                            } else {
                                                i - 1
                                            }
                                        }
                                        None => 0,
                                    };
                                    app.table_state.select(Some(i));
                                }
                                KeyCode::Down => {
                                    let i = match app.table_state.selected() {
                                        Some(i) => (i + 1) % 5,
                                        None => 0,
                                    };
                                    app.table_state.select(Some(i));
                                }
                                KeyCode::Char('s') => {
                                    app.log_autoscroll = !app.log_autoscroll;
                                }
                                _ => {}
                            },
                            ActiveTab::Scratchpad => match key.code {
                                KeyCode::Char('g') => app.regenerate_password(),
                                KeyCode::Char('[') => {
                                    if app.pass_length > 8 {
                                        app.pass_length -= 2;
                                        app.regenerate_password();
                                    }
                                }
                                KeyCode::Char(']') => {
                                    if app.pass_length < 64 {
                                        app.pass_length += 2;
                                        app.regenerate_password();
                                    }
                                }
                                KeyCode::Char('u') => {
                                    app.pass_upper = !app.pass_upper;
                                    app.regenerate_password();
                                }
                                KeyCode::Char('d') => {
                                    app.pass_digits = !app.pass_digits;
                                    app.regenerate_password();
                                }
                                KeyCode::Char('x') => {
                                    app.pass_symbols = !app.pass_symbols;
                                    app.regenerate_password();
                                }
                                KeyCode::Char('w') => {
                                    app.pass_diceware = !app.pass_diceware;
                                    app.regenerate_password();
                                }
                                KeyCode::Backspace => {
                                    app.scratchpad_text.pop();
                                }
                                KeyCode::Char(c) => {
                                    if !key.modifiers.contains(KeyModifiers::CONTROL) {
                                        app.scratchpad_text.push(c);
                                    }
                                }
                                KeyCode::Enter => {
                                    app.scratchpad_text.push('\n');
                                }
                                _ => {}
                            },
                        },
                    }
                }
            }
        }

        if app.last_tick.elapsed() >= tick_rate {
            app.update();
            app.last_tick = Instant::now();
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn ui(f: &mut Frame, app: &mut AppState) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Top Banner
            Constraint::Min(10),   // Content Viewport
            Constraint::Length(3), // Bottom Status Bar
        ])
        .split(f.area());

    // 1. TOP BANNER
    render_header(f, app, main_layout[0]);

    // 2. TAB CONTENT
    match app.tab {
        ActiveTab::KineticLab => render_kinetic_lab(f, app, main_layout[1]),
        ActiveTab::SecurityTelemetry => render_security_telemetry(f, app, main_layout[1]),
        ActiveTab::DataMatrix => render_data_matrix(f, app, main_layout[1]),
        ActiveTab::Scratchpad => render_scratchpad(f, app, main_layout[1]),
    }

    // 3. BOTTOM STATUS BAR
    render_footer(f, app, main_layout[2]);

    // MODAL POPUP HELP
    if app.show_help {
        render_help_modal(f, app);
    }
}

fn render_header(f: &mut Frame, app: &AppState, area: Rect) {
    let titles = vec![
        " 1: 🌀 Kinetic Fidget ",
        " 2: 🛡️ Security Telemetry ",
        " 3: 📊 Data Matrix ",
        " 4: ✍️ Crypto Pad & Gen ",
    ];

    let tabs = Tabs::new(titles)
        .select(app.tab as usize)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Line::from(vec![
                    Span::styled(" CASTELLUM ", Style::default().fg(Color::Black).bg(Color::Cyan).bold()),
                    Span::styled(" 🔒 SECURE TERMINAL WORKBENCH ", Style::default().fg(Color::Cyan).bold()),
                    Span::styled(" [Debian Linux 6.12] ", Style::default().fg(Color::DarkGray)),
                ])),
        )
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));

    f.render_widget(tabs, area);
}

fn render_footer(f: &mut Frame, app: &AppState, area: Rect) {
    let elapsed = app.start_time.elapsed().as_secs();
    let current_time = Local::now().format("%H:%M:%S").to_string();

    let footer_text = Line::from(vec![
        Span::styled(" [Tab/1-4] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Switch Tab │ "),
        Span::styled(" [?] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Help │ "),
        Span::styled(" [q] ", Style::default().fg(Color::Red).bold()),
        Span::raw("Exit │ "),
        Span::styled(format!(" ⏱ Uptime: {:02}:{:02} ", elapsed / 60, elapsed % 60), Style::default().fg(Color::Green)),
        Span::raw("│ "),
        Span::styled(format!(" 🕒 {} ", current_time), Style::default().fg(Color::Cyan)),
        Span::raw("│ "),
        Span::styled(
            format!(" ⚡ Tick: {} ", app.tick_count),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let p = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(block);

    f.render_widget(p, area);
}

// =========================================================================
// TAB 1: KINETIC FIDGET LAB
// =========================================================================
fn render_kinetic_lab(f: &mut Frame, app: &mut AppState, area: Rect) {
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(12), Constraint::Length(6)])
        .split(h_chunks[0]);

    // CANVAS SPINNER / ROTOR / 3D CUBE
    let angle = app.angle;
    let mode = app.kinetic_mode;
    let arms = app.spinner_arms;
    let particles = &app.particles;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Magenta))
                .title(format!(
                    " 🌀 Kinetic Gyroscope & Physics Canvas [Mode: {:?}] [Arms: {}] [Space: Spin, +/-: Speed, m: Mode, g: Gravity] ",
                    mode, arms
                )),
        )
        .x_bounds([-50.0, 50.0])
        .y_bounds([-50.0, 50.0])
        .paint(move |ctx| {
            // Draw coordinate crosshair faintly
            ctx.draw(&CanvasLine {
                x1: -48.0,
                y1: 0.0,
                x2: 48.0,
                y2: 0.0,
                color: Color::Rgb(40, 40, 50),
            });
            ctx.draw(&CanvasLine {
                x1: 0.0,
                y1: -48.0,
                x2: 0.0,
                y2: 48.0,
                color: Color::Rgb(40, 40, 50),
            });

            // Outer ring
            ctx.draw(&Circle {
                x: 0.0,
                y: 0.0,
                radius: 36.0,
                color: Color::Rgb(60, 60, 100),
            });

            match mode {
                KineticMode::Spinner => {
                    // Center core
                    ctx.draw(&Circle {
                        x: 0.0,
                        y: 0.0,
                        radius: 8.0,
                        color: Color::Yellow,
                    });

                    // Multi-arm spinning rotor
                    for i in 0..arms {
                        let arm_ang = angle + (i as f64 * 2.0 * PI / arms as f64);
                        let tip_x = arm_ang.cos() * 32.0;
                        let tip_y = arm_ang.sin() * 32.0;

                        ctx.draw(&CanvasLine {
                            x1: 0.0,
                            y1: 0.0,
                            x2: tip_x,
                            y2: tip_y,
                            color: Color::Cyan,
                        });

                        ctx.draw(&Circle {
                            x: tip_x,
                            y: tip_y,
                            radius: 5.5,
                            color: Color::LightMagenta,
                        });
                    }
                }
                KineticMode::Cube3D => {
                    // Rotating 3D wireframe hypercube projection
                    let size = 20.0;
                    let vertices = [
                        (-1.0, -1.0, -1.0),
                        (1.0, -1.0, -1.0),
                        (1.0, 1.0, -1.0),
                        (-1.0, 1.0, -1.0),
                        (-1.0, -1.0, 1.0),
                        (1.0, -1.0, 1.0),
                        (1.0, 1.0, 1.0),
                        (-1.0, 1.0, 1.0),
                    ];
                    let edges = [
                        (0, 1), (1, 2), (2, 3), (3, 0),
                        (4, 5), (5, 6), (6, 7), (7, 4),
                        (0, 4), (1, 5), (2, 6), (3, 7),
                    ];

                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    let cos_b = (angle * 0.7).cos();
                    let sin_b = (angle * 0.7).sin();

                    let projected: Vec<(f64, f64)> = vertices
                        .iter()
                        .map(|&(x, y, z)| {
                            let x1 = x * cos_a - z * sin_a;
                            let z1 = x * sin_a + z * cos_a;
                            let y2 = y * cos_b - z1 * sin_b;
                            (x1 * size, y2 * size)
                        })
                        .collect();

                    for &(u, v) in &edges {
                        ctx.draw(&CanvasLine {
                            x1: projected[u].0,
                            y1: projected[u].1,
                            x2: projected[v].0,
                            y2: projected[v].1,
                            color: Color::LightGreen,
                        });
                    }
                }
                KineticMode::Lissajous => {
                    // Hypnotic Lissajous Knot
                    let points: Vec<(f64, f64)> = (0..200)
                        .map(|t| {
                            let t_f = t as f64 * 2.0 * PI / 200.0;
                            let x = (3.0 * t_f + angle).sin() * 32.0;
                            let y = (4.0 * t_f).cos() * 32.0;
                            (x, y)
                        })
                        .collect();
                    ctx.draw(&Points {
                        coords: &points,
                        color: Color::Yellow,
                    });
                }
            }

            // Draw bouncing particles
            for p in particles {
                ctx.draw(&Points {
                    coords: &[(p.x, p.y)],
                    color: p.color,
                });
            }
        });

    f.render_widget(canvas, left_chunks[0]);

    // SPINNER TELEMETRY & STRESS GAUGE
    let speed_rpm = (app.angular_velocity.abs() * 60.0 / (2.0 * PI)) as u64;
    let gauge_val = (app.angular_velocity.abs() * 5.0).min(100.0) as u16;

    let kinetic_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(format!(" 🏎️ Kinetic Velocity: {} RPM (Stress Bar: {}%) ", speed_rpm, app.stress_level)),
        )
        .gauge_style(
            Style::default()
                .fg(if gauge_val > 70 { Color::Red } else if gauge_val > 40 { Color::Yellow } else { Color::Cyan })
                .bg(Color::DarkGray),
        )
        .percent(gauge_val)
        .label(format!("Speed: {}% | Stress: {}%", gauge_val, app.stress_level));

    f.render_widget(kinetic_gauge, left_chunks[1]);

    // RIGHT HALF: TACTILE BUBBLE WRAP / CLICKER
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(h_chunks[1]);

    // Bubble Grid Matrix
    let mut grid_lines = Vec::new();
    grid_lines.push(Line::from(vec![
        Span::styled(" TACTILE BUBBLE WRAP / MECHANICAL SWITCH MATRIX ", Style::default().fg(Color::Yellow).bold()),
    ]));
    grid_lines.push(Line::from(vec![
        Span::styled(" [Arrows: Move] [Enter: POP] [Goal: Relieve All Tension]", Style::default().fg(Color::DarkGray)),
    ]));
    grid_lines.push(Line::raw(""));

    for r in 0..4 {
        let mut row_spans = Vec::new();
        row_spans.push(Span::raw("   "));
        for c in 0..4 {
            let is_cur = (r, c) == app.bubble_cursor;
            let popped = app.bubble_grid[r][c];

            let cell_str = if popped {
                "[  *  ]"
            } else {
                "[ (o) ]"
            };

            let mut style = if popped {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::LightGreen).bold()
            };

            if is_cur {
                style = style.bg(Color::Rgb(60, 60, 120)).fg(Color::Yellow).add_modifier(Modifier::BOLD);
            }

            row_spans.push(Span::styled(cell_str, style));
            row_spans.push(Span::raw(" "));
        }
        grid_lines.push(Line::from(row_spans));
        grid_lines.push(Line::raw(""));
    }

    grid_lines.push(Line::from(vec![
        Span::styled(" Last Feedback: ", Style::default().fg(Color::DarkGray)),
        Span::styled(&app.pop_feedback, Style::default().fg(Color::LightMagenta).bold()),
    ]));

    let bubble_block = Paragraph::new(grid_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" 🔘 Tactile Stress Reliever Clicker "),
        )
        .alignment(Alignment::Center);

    f.render_widget(bubble_block, right_chunks[0]);

    // STRESS RELIEF METRICS
    let stress_color = if app.stress_level > 60 {
        Color::Red
    } else if app.stress_level > 30 {
        Color::Yellow
    } else {
        Color::Green
    };

    let stress_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" 💆 User Stress Level Index "),
        )
        .gauge_style(Style::default().fg(stress_color).bg(Color::DarkGray))
        .percent(app.stress_level)
        .label(format!("Stress: {}% (Total Pops: {})", app.stress_level, app.bubbles_popped));

    f.render_widget(stress_gauge, right_chunks[1]);
}

// =========================================================================
// TAB 2: SECURITY & MEMORY TELEMETRY
// =========================================================================
fn render_security_telemetry(f: &mut Frame, app: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Argon2id & TOTP
            Constraint::Length(10), // Entropy Live Sparkline & Spectrum
            Constraint::Min(8),     // memfd_secret Memory Map
        ])
        .split(area);

    // TOP: ARGON2ID & TOTP
    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    // Argon2id KDF Simulation
    let argon_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" 🔐 Argon2id Memory-Hard KDF (RFC 9106) [m: 256MB, t: 5, p: 4] "),
        )
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::DarkGray))
        .percent(app.argon_progress)
        .label(format!("KDF Hash Progress: {}% (K_ikm derivation)", app.argon_progress));

    f.render_widget(argon_gauge, top_chunks[0]);

    // TOTP RFC 6238 Token Clock
    let _totp_percent = (app.totp_countdown as u16 * 100) / 30;
    let totp_color = if app.totp_countdown <= 5 {
        Color::Red
    } else if app.totp_countdown <= 12 {
        Color::Yellow
    } else {
        Color::Green
    };

    let totp_lines = vec![
        Line::from(vec![
            Span::styled(" ACTIVE OTP CODE:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:03} {:03}", app.totp_code / 1000, app.totp_code % 1000),
                Style::default().fg(Color::Yellow).bold().add_modifier(Modifier::RAPID_BLINK),
            ),
            Span::styled(format!("  [Expires in: {:02}s]", app.totp_countdown), Style::default().fg(totp_color)),
        ]),
    ];

    let totp_p = Paragraph::new(totp_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Magenta))
                .title(" 🔑 RFC 6238 TOTP Multi-Factor Generator "),
        )
        .alignment(Alignment::Center);

    f.render_widget(totp_p, top_chunks[1]);

    // MIDDLE: SHANNON ENTROPY SPARKLINE & LIVE STREAM
    let entropy_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(chunks[1]);

    let sparkline = Sparkline::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::LightGreen))
                .title(" 📈 Live Container Shannon Entropy Stream H (Target: H >= 7.99 bits/byte) [e: Toggle CSPRNG/Low] "),
        )
        .data(&app.entropy_history)
        .max(800)
        .style(Style::default().fg(if app.current_entropy >= 7.95 { Color::Green } else { Color::Red }));

    f.render_widget(sparkline, entropy_chunks[0]);

    let entropy_status = vec![
        Line::from(vec![
            Span::styled("Current H: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.3} bits/B", app.current_entropy),
                Style::default().fg(if app.current_entropy >= 7.95 { Color::Green } else { Color::Red }).bold(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Stream Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if app.high_entropy_stream { "ChaCha20 CSPRNG" } else { "Structured Text" },
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Deniability: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if app.current_entropy >= 7.95 { "Plausible" } else { "Detectable" },
                Style::default().fg(if app.current_entropy >= 7.95 { Color::Green } else { Color::Red }).bold(),
            ),
        ]),
    ];

    let entropy_p = Paragraph::new(entropy_status)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" 🛡️ Classification "),
        )
        .alignment(Alignment::Left);

    f.render_widget(entropy_p, entropy_chunks[1]);

    // BOTTOM: MEMFD_SECRET KERNEL MEMORY MAP
    let mut mem_spans = Vec::new();
    mem_spans.push(Span::styled(
        " Linux Kernel Direct-Map Unmapped Secret Pages (memfd_secret syscall 447) [Press 'w' to Panic Wipe]:\n\n ",
        Style::default().fg(Color::Yellow).bold(),
    ));

    for (i, (tag, color)) in app.memory_blocks.iter().enumerate() {
        mem_spans.push(Span::styled(format!("[{}] ", tag), Style::default().fg(*color).bold()));
        if (i + 1) % 16 == 0 {
            mem_spans.push(Span::raw("\n "));
        }
    }

    let mem_p = Paragraph::new(Line::from(mem_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Blue))
                .title(" 🧠 Hardware/Kernel Memory Map Isolation Inspection "),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(mem_p, chunks[2]);
}

// =========================================================================
// TAB 3: DATA MATRIX & LOG STREAM
// =========================================================================
fn render_data_matrix(f: &mut Frame, app: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    // VAULT RECORD TABLE
    let header = Row::new(vec![
        "ID",
        "SERVICE / SYSTEM",
        "USERNAME / IDENTITY",
        "SECURITY LEVEL",
        "CIPHER SUITE",
        "STATUS",
    ])
    .style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )
    .height(1);

    let rows = vec![
        Row::new(vec![
            "001",
            "Root SSH (Infra-01)",
            "admin@castellum.internal",
            "Paranoid (Argon2id-1GB)",
            "XChaCha20-Poly1305",
            "LOCKED (memfd_secret)",
        ]),
        Row::new(vec![
            "002",
            "ProtonMail (Primary)",
            "serhii.prilepskyi@pm.me",
            "High (Argon2id-256MB)",
            "XChaCha20-Poly1305",
            "ZEROIZED",
        ]),
        Row::new(vec![
            "003",
            "GitHub Enterprise PAT",
            "Abendgast",
            "High (Argon2id-256MB)",
            "XChaCha20-Poly1305",
            "ACTIVE SESSION",
        ]),
        Row::new(vec![
            "004",
            "Decoy Cloud Backup",
            "johndoe@gmail.com",
            "Duress (Decoy Slot 1)",
            "XChaCha20-Poly1305",
            "PLAUSIBLE DECOY",
        ]),
        Row::new(vec![
            "005",
            "PGP Master Signing Key",
            "4096R/0x8F94A1C3",
            "Paranoid (Argon2id-1GB)",
            "XChaCha20-Poly1305",
            "MLOCKALL PINNED",
        ]),
    ];

    let widths = [
        Constraint::Length(5),
        Constraint::Length(25),
        Constraint::Length(28),
        Constraint::Length(25),
        Constraint::Length(22),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" 📁 Castellum Encrypted Vault Entries [Up/Down: Select Row] "),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::Rgb(40, 70, 110))
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, chunks[0], &mut app.table_state);

    // LIVE LOG CONSOLE
    let log_items: Vec<ListItem> = app
        .log_messages
        .iter()
        .map(|(ts, msg, color)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", ts), Style::default().fg(Color::DarkGray)),
                Span::styled(*msg, Style::default().fg(*color)),
            ]))
        })
        .collect();

    let logs_list = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green))
            .title(format!(
                " 📜 System Security & Audit Log Stream [Auto-scroll: {} (s: Toggle)] ",
                if app.log_autoscroll { "ON" } else { "PAUSED" }
            )),
    );

    f.render_widget(logs_list, chunks[1]);
}

// =========================================================================
// TAB 4: CRYPTO SCRATCHPAD & PASSWORD GENERATOR
// =========================================================================
fn render_scratchpad(f: &mut Frame, app: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // LEFT: PASSWORD GENERATOR
    let mut gen_lines = Vec::new();
    gen_lines.push(Line::from(vec![
        Span::styled(" INDUSTRIAL CRYPTOGRAPHIC PASSWORD GENERATOR ", Style::default().fg(Color::Yellow).bold()),
    ]));
    gen_lines.push(Line::raw(""));

    gen_lines.push(Line::from(vec![
        Span::styled(" Generated Value: ", Style::default().fg(Color::DarkGray)),
    ]));
    gen_lines.push(Line::from(vec![
        Span::styled(
            format!(" >> {} << ", app.generated_pass),
            Style::default().fg(Color::Black).bg(Color::Green).bold(),
        ),
    ]));
    gen_lines.push(Line::raw(""));

    let bit_entropy = if app.pass_diceware {
        4.0 * 12.9
    } else {
        let mut pool: f64 = 26.0;
        if app.pass_upper { pool += 26.0; }
        if app.pass_digits { pool += 10.0; }
        if app.pass_symbols { pool += 32.0; }
        app.pass_length as f64 * pool.log2()
    };

    gen_lines.push(Line::from(vec![
        Span::styled(" Calculated Shannon Entropy: ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{:.1} bits ", bit_entropy), Style::default().fg(Color::Cyan).bold()),
        Span::styled(
            if bit_entropy > 120.0 {
                " [MILITARY GRADE - UNBREAKABLE]"
            } else if bit_entropy > 80.0 {
                " [STRONG]"
            } else {
                " [MODERATE]"
            },
            Style::default().fg(Color::Green).bold(),
        ),
    ]));
    gen_lines.push(Line::raw(""));

    gen_lines.push(Line::from(vec![
        Span::styled(" [g] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw("Regenerate Password"),
    ]));
    gen_lines.push(Line::from(vec![
        Span::styled(" [[ / ]] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!("Length: {} chars", app.pass_length)),
    ]));
    gen_lines.push(Line::from(vec![
        Span::styled(" [u] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!("Uppercase [A-Z]: {}", if app.pass_upper { "YES" } else { "NO" })),
    ]));
    gen_lines.push(Line::from(vec![
        Span::styled(" [d] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!("Digits [0-9]: {}", if app.pass_digits { "YES" } else { "NO" })),
    ]));
    gen_lines.push(Line::from(vec![
        Span::styled(" [x] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!("Symbols [!@#$]: {}", if app.pass_symbols { "YES" } else { "NO" })),
    ]));
    gen_lines.push(Line::from(vec![
        Span::styled(" [w] ", Style::default().fg(Color::Yellow).bold()),
        Span::raw(format!("Diceware Passphrase: {}", if app.pass_diceware { "ACTIVE" } else { "DISABLED" })),
    ]));

    let gen_block = Paragraph::new(gen_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" 🎲 CSPRNG Password & Passphrase Engine "),
        )
        .alignment(Alignment::Left);

    f.render_widget(gen_block, chunks[0]);

    // RIGHT: SECURE INLINE SCRATCHPAD
    let scratch_lines = vec![
        Line::from(vec![
            Span::styled(" SECURE VOLATILE SCRATCHPAD (TYPE DIRECTLY) ", Style::default().fg(Color::Cyan).bold()),
        ]),
        Line::from(vec![
            Span::styled(" [All typed data is wiped with zeroize on program exit]", Style::default().fg(Color::DarkGray)),
        ]),
        Line::raw(""),
        Line::styled(&app.scratchpad_text, Style::default().fg(Color::White)),
        Line::styled("█", Style::default().fg(Color::Cyan).add_modifier(Modifier::RAPID_BLINK)),
    ];

    let scratch_p = Paragraph::new(scratch_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" 📝 Ephemeral In-Memory Note Pad "),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });

    f.render_widget(scratch_p, chunks[1]);
}

// =========================================================================
// HELP MODAL OVERLAY
// =========================================================================
fn render_help_modal(f: &mut Frame, _app: &AppState) {
    let area = f.area();
    let popup_width = 70.min(area.width.saturating_sub(4));
    let popup_height = 22.min(area.height.saturating_sub(4));

    let popup_area = Rect::new(
        (area.width.saturating_sub(popup_width)) / 2,
        (area.height.saturating_sub(popup_height)) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(vec![
            Span::styled(" CASTELLUM TERMINAL WORKBENCH - KEYBOARD CONTROLS ", Style::default().fg(Color::Yellow).bold()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" [Tab] / [1-4]    ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Switch between Active Tabs"),
        ]),
        Line::from(vec![
            Span::styled(" [Space] / [+]   ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Spin Fidget Gyroscope / Accelerate Kinetic Rotor"),
        ]),
        Line::from(vec![
            Span::styled(" [-] / [r]       ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Brake Rotor / Reverse Rotation Direction"),
        ]),
        Line::from(vec![
            Span::styled(" [m] / [a]       ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Switch Kinetic Mode (Rotor ↔ 3D Cube ↔ Lissajous) / Change Arms"),
        ]),
        Line::from(vec![
            Span::styled(" [Arrows / Enter]", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Navigate Tactile Bubble Wrap Matrix & Pop Cells"),
        ]),
        Line::from(vec![
            Span::styled(" [g]             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Toggle Gravity in Physics Box / Regenerate Password"),
        ]),
        Line::from(vec![
            Span::styled(" [w]             ", Style::default().fg(Color::Red).bold()),
            Span::raw("Trigger Simulated Panic Memory Wipe (Zeroization)"),
        ]),
        Line::from(vec![
            Span::styled(" [e]             ", Style::default().fg(Color::Green).bold()),
            Span::raw("Toggle Live Entropy Stream (CSPRNG vs Low Text)"),
        ]),
        Line::from(vec![
            Span::styled(" [? / h]         ", Style::default().fg(Color::Yellow).bold()),
            Span::raw("Toggle this Help Overlay"),
        ]),
        Line::from(vec![
            Span::styled(" [q / Esc]       ", Style::default().fg(Color::Red).bold()),
            Span::raw("Exit Application safely"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Press [?] or [Esc] to close this help dialog.", Style::default().fg(Color::DarkGray).italic()),
        ]),
    ];

    let block = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" ❓ Workbench Help & Keybindings "),
        )
        .alignment(Alignment::Left);

    f.render_widget(block, popup_area);
}
