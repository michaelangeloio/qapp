use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{
    io::stdout,
    process::{Command, Stdio},
    time::Duration,
};

// App icons for common applications
const APP_ICONS: &[(&str, &str)] = &[
    // Browsers
    ("Safari", "🌐"),
    ("Firefox", "🦊"),
    ("Chrome", "🌐"),
    ("Edge", "🌐"),
    ("Microsoft Edge", "🌐"),
    ("Arc", "🌍"),
    
    // Terminals
    ("Terminal", "💻"),
    ("iTerm", "💻"),
    ("iTerm2", "💻"),
    ("Warp", "🚀"),
    ("kitty", "🐱"),
    ("Ghostty", "👻"),
    
    // System utilities
    ("Finder", "📁"),
    ("System Settings", "⚙️"),
    ("Activity Monitor", "📊"),
    ("Memory Diag", "🧠"),
    ("App Store", "🛍️"),
    ("Font Book", "🔤"),
    ("Keychain", "🔑"),
    ("Paste", "📋"),
    ("Magnet", "🧲"),
    ("Windsurf", "🏄"),
    ("keymapp", "⌨️"),
    
    // Productivity & Development
    ("Visual Studio Code", "💻"),
    ("Xcode", "🛠️"),
    ("Cursor", "📝"),
    ("Rancher Desktop", "🐮"),
    ("Docker", "🐳"),
    ("Postgres", "🐘"),
    ("DB Browser for SQLite", "🗄️"),
    ("pgAdmin", "🐘"),
    ("Lens", "🔍"),
    ("Authy", "🔐"),
    ("1Password", "🔐"),
    ("Github", "🐙"),
    ("HubAI", "🧠"),
    ("Repo Prompt", "💬"),
    
    // Creative apps
    ("Final Cut Pro", "🎬"),
    ("iMovie", "🎥"),
    ("GarageBand", "🎸"),
    ("Numbers", "🔢"),
    ("Pages", "📄"),
    ("Keynote", "📊"),
    ("Insta360", "📸"),
    
    // Communication
    ("Mail", "✉️"),
    ("Messages", "💬"),
    ("Slack", "💬"),
    ("Discord", "💬"),
    ("Klack", "⌨️"),
    ("Zoom", "🎦"),
    ("zoom.us", "🎦"),
    ("FaceTime", "📹"),
    ("Claude", "🧠"),
    ("Notion", "📝"),
    ("Copilot", "🤖"),
    
    // Media
    ("Music", "🎵"),
    ("Spotify", "🎵"),
    ("Photos", "🖼️"),
    ("Preview", "👁️"),
    ("Books", "📚"),
    
    // Utilities
    ("Calendar", "📅"),
    ("Notes", "📝"),
    ("Calculator", "🧮"),
    ("Maps", "🗺️"),
    ("Reminders", "📋"),
    ("Siri", "🔍"),
    ("TextEdit", "📄"),
    ("TestFlight", "✈️"),
    
    // VPN & Security
    ("ExpressVPN", "🔒"),
    ("AWS VPN Client", "🔒"),
    ("VPN", "🔒"),
];

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all open applications
    List,
    /// Open an application
    Open {
        /// The application name to open (without .app)
        name: Option<String>,
    },
    /// Kill (terminate) an application
    Kill {
        /// The application name to kill
        name: Option<String>,
    },
}

enum Mode {
    Normal,
    Search,
}

enum ActionStatus {
    None,
    Opened(String),
    Killed(String),
}

struct AppState {
    apps: Vec<String>,
    installed_apps: Vec<String>,
    filtered_apps: Vec<String>,
    selected_index: usize,
    mode: Mode,
    search_query: String,
    should_quit: bool,
    action_status: ActionStatus,
    status_counter: u8,
}

impl AppState {
    fn new(running_apps: Vec<String>) -> Self {
        Self {
            apps: running_apps,
            installed_apps: Vec::new(),
            filtered_apps: Vec::new(),
            selected_index: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            should_quit: false,
            action_status: ActionStatus::None,
            status_counter: 0,
        }
    }
    
    fn set_opened(&mut self, app_name: String) {
        self.action_status = ActionStatus::Opened(app_name);
        self.status_counter = 30; // Show for ~3 seconds (at 100ms per frame)
    }
    
    fn set_killed(&mut self, app_name: String) {
        self.action_status = ActionStatus::Killed(app_name);
        self.status_counter = 30; // Show for ~3 seconds
    }
    
    fn update_status(&mut self) {
        if self.status_counter > 0 {
            self.status_counter -= 1;
            if self.status_counter == 0 {
                self.action_status = ActionStatus::None;
            }
        }
    }

    fn load_installed_apps(&mut self) -> Result<()> {
        // Get list of installed applications
        let output = Command::new("find")
            .args(["/Applications", "-maxdepth", "2", "-name", "*.app"])
            .output()
            .context("Failed to list installed applications")?;

        let output_str = String::from_utf8(output.stdout)
            .context("Failed to parse find output")?;

        self.installed_apps = output_str
            .lines()
            .map(|s| {
                s.trim()
                    .strip_prefix("/Applications/")
                    .unwrap_or(s)
                    .strip_suffix(".app")
                    .unwrap_or(s)
                    .to_string()
            })
            .collect();

        self.filter_installed_apps();
        Ok(())
    }

    fn filter_installed_apps(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_apps = self.installed_apps.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_apps = self.installed_apps
                .iter()
                .filter(|app| app.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
        // Reset selection if filtered list changes
        if !self.filtered_apps.is_empty() {
            self.selected_index = self.selected_index.min(self.filtered_apps.len() - 1);
        } else {
            self.selected_index = 0;
        }
    }

    fn next(&mut self) {
        let apps = match self.mode {
            Mode::Normal => &self.apps,
            Mode::Search => &self.filtered_apps,
        };
        
        let len = apps.len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    fn previous(&mut self) {
        let apps = match self.mode {
            Mode::Normal => &self.apps,
            Mode::Search => &self.filtered_apps,
        };
        
        let len = apps.len();
        if len > 0 {
            self.selected_index = (self.selected_index + len - 1) % len;
        }
    }

    fn selected_app(&self) -> Option<&String> {
        match self.mode {
            Mode::Normal => self.apps.get(self.selected_index),
            Mode::Search => self.filtered_apps.get(self.selected_index),
        }
    }

    fn add_to_search(&mut self, c: char) {
        self.search_query.push(c);
        self.filter_installed_apps();
    }

    fn backspace_search(&mut self) {
        self.search_query.pop();
        self.filter_installed_apps();
    }

    fn enter_search_mode(&mut self) -> Result<()> {
        if self.installed_apps.is_empty() {
            self.load_installed_apps()?;
        }
        
        self.mode = Mode::Search;
        self.search_query.clear();
        self.filter_installed_apps();
        self.selected_index = 0;
        Ok(())
    }

    fn exit_search_mode(&mut self) {
        self.mode = Mode::Normal;
        self.search_query.clear();
        self.selected_index = 0;
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::List) => interactive_app_list()?,
        Some(Commands::Open { name }) => open_application(name)?,
        Some(Commands::Kill { name }) => kill_application(name)?,
        None => interactive_app_list()?,
    }

    Ok(())
}

fn get_running_applications() -> Result<Vec<String>> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of (processes where background only is false)")
        .output()
        .context("Failed to execute osascript command")?;

    let output_str = String::from_utf8(output.stdout)
        .context("Failed to parse osascript output")?;

    // Parse the AppleScript output format
    let apps: Vec<String> = output_str
        .trim()
        .trim_matches(|c| c == '{' || c == '}')
        .split(", ")
        .map(|s| s.trim_matches('"').to_string())
        .collect();

    Ok(apps)
}

fn get_app_icon(app_name: &str) -> &'static str {
    for (name, icon) in APP_ICONS {
        if app_name.contains(name) {
            return icon;
        }
    }
    "📱" // Default icon for applications
}

fn interactive_app_list() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Get running apps
    let apps = get_running_applications()?;
    
    if apps.is_empty() {
        // Clean up terminal
        terminal::disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        
        println!("{}", "No visible applications found.".yellow());
        return Ok(());
    }
    
    // Create app state
    let mut app_state = AppState::new(apps);
    
    // Preload installed apps in the background
    app_state.load_installed_apps()?;

    // Application loop
    while !app_state.should_quit {
        terminal.draw(|frame| {
            // Create layout for the UI
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Min(5),     // List of apps
                    Constraint::Length(3),  // Footer
                ])
                .split(frame.area());

            // Header - changes based on mode
            let header_text = match app_state.mode {
                Mode::Normal => "Running Applications",
                Mode::Search => "Search Applications",
            };
            
            let header_content = match app_state.mode {
                Mode::Normal => Line::from(vec![
                    Span::styled(
                        header_text,
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    )
                ]),
                Mode::Search => Line::from(vec![
                    Span::styled(
                        format!("{}: ", header_text),
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &app_state.search_query,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "_",  // Cursor
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ),
                ]),
            };
            
            let header = Paragraph::new(header_content)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            
            frame.render_widget(header, chunks[0]);

            // List of apps - changes based on mode
            let app_items: Vec<ListItem> = match app_state.mode {
                Mode::Normal => app_state.apps
                    .iter()
                    .enumerate()
                    .map(|(i, app)| {
                        let icon = get_app_icon(app);
                        let content = Line::from(vec![
                            Span::raw(format!("{} ", icon)),
                            Span::styled(
                                app.clone(), 
                                Style::default().fg(if i == app_state.selected_index { 
                                    Color::Yellow 
                                } else { 
                                    Color::White 
                                })
                            ),
                        ]);
                        ListItem::new(content)
                    })
                    .collect(),
                Mode::Search => app_state.filtered_apps
                    .iter()
                    .enumerate()
                    .map(|(i, app)| {
                        let icon = get_app_icon(app);
                        let content = Line::from(vec![
                            Span::raw(format!("{} ", icon)),
                            Span::styled(
                                app.clone(), 
                                Style::default().fg(if i == app_state.selected_index { 
                                    Color::Yellow 
                                } else { 
                                    Color::White 
                                })
                            ),
                        ]);
                        ListItem::new(content)
                    })
                    .collect(),
            };

            let list_title = match app_state.mode {
                Mode::Normal => "Running Applications",
                Mode::Search => if app_state.filtered_apps.is_empty() {
                    "No matching applications"
                } else {
                    "Matching Applications"
                },
            };

            let apps_list = List::new(app_items)
                .block(Block::default()
                    .title(list_title)
                    .borders(Borders::ALL))
                .highlight_style(
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("➤ ");

            frame.render_stateful_widget(
                apps_list,
                chunks[1],
                &mut ratatui::widgets::ListState::default().with_selected(Some(app_state.selected_index)),
            );

            // Footer with keybindings or status message
            let footer_content = match app_state.action_status {
                ActionStatus::None => {
                    // Show normal keybindings
                    let keybindings = match app_state.mode {
                        Mode::Normal => "↑/↓: Navigate   O: Open   K: Kill   /: Search   Q: Quit",
                        Mode::Search => "↑/↓: Navigate   Enter: Open   Esc: Cancel   Backspace: Delete",
                    };
                    
                    Line::from(vec![
                        Span::styled(
                            keybindings, 
                            Style::default().fg(Color::Yellow)
                        )
                    ])
                },
                ActionStatus::Opened(ref app_name) => {
                    // Show opened confirmation
                    Line::from(vec![
                        Span::styled(
                            "✅ ", 
                            Style::default().fg(Color::Green)
                        ),
                        Span::styled(
                            app_name.clone(),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                        ),
                        Span::styled(
                            " opened",
                            Style::default().fg(Color::Green)
                        )
                    ])
                },
                ActionStatus::Killed(ref app_name) => {
                    // Show killed confirmation
                    Line::from(vec![
                        Span::styled(
                            "❌ ", 
                            Style::default().fg(Color::Red)
                        ),
                        Span::styled(
                            app_name.clone(),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
                        ),
                        Span::styled(
                            " terminated",
                            Style::default().fg(Color::Red)
                        )
                    ])
                },
            };
            
            let footer = Paragraph::new(footer_content)
                .alignment(Alignment::Left)
                .block(Block::default().borders(Borders::ALL));
            
            frame.render_widget(footer, chunks[2]);
        })?;

        // Update action status counter
        app_state.update_status();

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, modifiers, kind, .. }) = event::read()? {
                if kind == KeyEventKind::Press {
                    match app_state.mode {
                        Mode::Normal => match code {
                            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
                                app_state.should_quit = true;
                            },
                            KeyCode::Up => {
                                app_state.previous();
                            },
                            KeyCode::Down => {
                                app_state.next();
                            },
                            KeyCode::Char('o') | KeyCode::Char('O') => {
                                if let Some(app_name) = app_state.selected_app() {
                                    let app_name_copy = app_name.clone();
                                    
                                    // Open the application
                                    open_specific_application(app_name)?;
                                    
                                    // Update state with success message
                                    app_state.set_opened(app_name_copy);
                                    
                                    // Refresh the list of running apps
                                    if let Ok(updated_apps) = get_running_applications() {
                                        app_state.apps = updated_apps;
                                        app_state.selected_index = app_state.selected_index.min(app_state.apps.len().saturating_sub(1));
                                    }
                                }
                            },
                            KeyCode::Char('k') | KeyCode::Char('K') => {
                                if let Some(app_name) = app_state.selected_app() {
                                    let app_name_copy = app_name.clone();
                                    
                                    // Kill the application
                                    kill_specific_application(app_name)?;
                                    
                                    // Update state with success message
                                    app_state.set_killed(app_name_copy);
                                    
                                    // Refresh the list of running apps
                                    if let Ok(updated_apps) = get_running_applications() {
                                        app_state.apps = updated_apps;
                                        app_state.selected_index = app_state.selected_index.min(app_state.apps.len().saturating_sub(1));
                                    }
                                }
                            },
                            KeyCode::Char('/') => {
                                app_state.enter_search_mode()?;
                            },
                            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                                app_state.should_quit = true;
                            },
                            _ => {}
                        },
                        Mode::Search => match code {
                            KeyCode::Esc => {
                                app_state.exit_search_mode();
                            },
                            KeyCode::Enter => {
                                if let Some(app_name) = app_state.selected_app() {
                                    let app_name_copy = app_name.clone();
                                    
                                    // Open the application
                                    open_specific_application(app_name)?;
                                    
                                    // Update state with success message and exit search mode
                                    app_state.set_opened(app_name_copy);
                                    app_state.exit_search_mode();
                                    
                                    // Refresh the list of running apps
                                    if let Ok(updated_apps) = get_running_applications() {
                                        app_state.apps = updated_apps;
                                        app_state.selected_index = app_state.selected_index.min(app_state.apps.len().saturating_sub(1));
                                    }
                                }
                            },
                            KeyCode::Backspace => {
                                app_state.backspace_search();
                            },
                            KeyCode::Up => {
                                app_state.previous();
                            },
                            KeyCode::Down => {
                                app_state.next();
                            },
                            KeyCode::Char(c) => {
                                if c == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                    app_state.should_quit = true;
                                } else {
                                    app_state.add_to_search(c);
                                }
                            },
                            _ => {}
                        },
                    }
                }
            }
        }
    }

    // Clean up terminal
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn open_specific_application(app_name: &str) -> Result<()> {
    // No need to print here since we show status in the UI
    Command::new("open")
        .arg("-a")
        .arg(app_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context(format!("Failed to open application: {}", app_name))?;

    Ok(())
}

fn kill_specific_application(app_name: &str) -> Result<()> {
    // No need to print here since we show status in the UI
    Command::new("osascript")
        .arg("-e")
        .arg(format!("tell application \"{}\" to quit", app_name))
        .output()
        .context(format!("Failed to kill application: {}", app_name))?;

    Ok(())
}

fn open_application(name: &Option<String>) -> Result<()> {
    match name {
        Some(name) => {
            // When using from command line, print a message
            println!("{} {}", "Opening:".green(), name.cyan());
            open_specific_application(name)
        },
        None => {
            // Setup terminal for interactive search
            terminal::enable_raw_mode()?;
            let mut stdout = stdout();
            execute!(stdout, EnterAlternateScreen)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;
            
            // Create app state in search mode
            let mut app_state = AppState::new(vec![]);
            app_state.load_installed_apps()?;
            app_state.enter_search_mode()?;
            
            // Application loop
            while !app_state.should_quit {
                terminal.draw(|frame| {
                    // Create layout for the UI
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(2)
                        .constraints([
                            Constraint::Length(3),  // Header
                            Constraint::Min(5),     // List of apps
                            Constraint::Length(3),  // Footer
                        ])
                        .split(frame.area());
        
                    // Header with search
                    let header = Paragraph::new(Line::from(vec![
                        Span::styled(
                            "Search Applications: ",
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            &app_state.search_query,
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "_",  // Cursor
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                        ),
                    ]))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                    
                    frame.render_widget(header, chunks[0]);
        
                    // List of matching apps
                    let list_title = if app_state.filtered_apps.is_empty() {
                        "No matching applications"
                    } else {
                        "Matching Applications"
                    };
                    
                    let app_items: Vec<ListItem> = app_state.filtered_apps
                        .iter()
                        .enumerate()
                        .map(|(i, app)| {
                            let icon = get_app_icon(app);
                            let content = Line::from(vec![
                                Span::raw(format!("{} ", icon)),
                                Span::styled(
                                    app.clone(), 
                                    Style::default().fg(if i == app_state.selected_index { 
                                        Color::Yellow 
                                    } else { 
                                        Color::White 
                                    })
                                ),
                            ]);
                            ListItem::new(content)
                        })
                        .collect();
        
                    let apps_list = List::new(app_items)
                        .block(Block::default()
                            .title(list_title)
                            .borders(Borders::ALL))
                        .highlight_style(
                            Style::default()
                                .bg(Color::Blue)
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("➤ ");
        
                    frame.render_stateful_widget(
                        apps_list,
                        chunks[1],
                        &mut ratatui::widgets::ListState::default().with_selected(Some(app_state.selected_index)),
                    );
        
                    // Footer with keybindings
                    let footer = Paragraph::new(Line::from(vec![
                        Span::styled(
                            "↑/↓: Navigate   Enter: Open   Esc: Cancel   Backspace: Delete", 
                            Style::default().fg(Color::Yellow)
                        )
                    ]))
                    .alignment(Alignment::Center)
                    .block(Block::default().borders(Borders::ALL));
                    
                    frame.render_widget(footer, chunks[2]);
                })?;
        
                // Handle input
                if event::poll(Duration::from_millis(100))? {
                    if let Event::Key(KeyEvent { code, modifiers, kind, .. }) = event::read()? {
                        if kind == KeyEventKind::Press {
                            match code {
                                KeyCode::Esc => {
                                    app_state.should_quit = true;
                                },
                                KeyCode::Enter => {
                                    if let Some(app_name) = app_state.selected_app() {
                                        // Clean up terminal
                                        terminal::disable_raw_mode()?;
                                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                                        terminal.show_cursor()?;
                                        
                                        open_specific_application(app_name)?;
                                        return Ok(());
                                    }
                                },
                                KeyCode::Backspace => {
                                    app_state.backspace_search();
                                },
                                KeyCode::Up => {
                                    app_state.previous();
                                },
                                KeyCode::Down => {
                                    app_state.next();
                                },
                                KeyCode::Char(c) => {
                                    if c == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                        app_state.should_quit = true;
                                    } else {
                                        app_state.add_to_search(c);
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
            
            // Clean up terminal
            terminal::disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            
            Ok(())
        }
    }
}

fn kill_application(name: &Option<String>) -> Result<()> {
    match name {
        Some(name) => {
            let apps = get_running_applications()?;
            
            if apps.is_empty() {
                println!("{}", "No running applications found.".yellow());
                return Ok(());
            }
            
            if !apps.contains(name) {
                println!("{} {}", "Application not running:".red(), name.cyan());
                return Ok(());
            }
            
            // When using from command line, print a message
            println!("{} {}", "Killing:".red(), name.cyan());
            kill_specific_application(name)
        },
        None => {
            // Use our interactive app list which already has the kill functionality
            interactive_app_list()
        }
    }
}
