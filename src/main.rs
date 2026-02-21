mod app;
mod config;
mod event;
mod api;
mod ui;
mod keybinds;
mod markdown;
mod neovim;
mod history;

use std::io;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use app::App;
use config::Config;
use event::EventHandler;

#[derive(Parser)]
#[command(name = "pro", about = "Fast TUI chat CLI with vim keybindings")]
struct Cli {
    /// Start with a prompt directly
    #[arg(short, long)]
    prompt: Option<String>,

    /// Model to use (e.g. claude-sonnet-4-20250514, gpt-4o)
    #[arg(short, long)]
    model: Option<String>,

    /// API provider (anthropic, openai)
    #[arg(long)]
    provider: Option<String>,

    /// Start in a specific conversation
    #[arg(short, long)]
    conversation: Option<String>,

    /// Neovim socket path for integration
    #[arg(long)]
    nvim_socket: Option<String>,

    /// Print config path and exit
    #[arg(long)]
    config_path: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set up file logging
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("pro-chat")
        .join("logs");
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(&log_dir, "pro-chat.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("pro_chat=debug".parse().unwrap()),
        )
        .init();

    let config = Config::load()?;

    if cli.config_path {
        println!("{}", Config::path().display());
        return Ok(());
    }

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(config);

    if let Some(model) = cli.model {
        app.set_model(&model);
    }
    if let Some(provider) = cli.provider {
        app.set_provider(&provider);
    }
    if let Some(conv) = cli.conversation {
        app.load_conversation(&conv)?;
    }
    if let Some(socket) = cli.nvim_socket {
        app.set_nvim_socket(&socket);
    }

    // If a prompt was given via CLI, send it immediately
    if let Some(prompt) = cli.prompt {
        app.set_input(&prompt);
        app.send_message().await?;
    }

    // Event handler
    let events = EventHandler::new(250);

    // Main loop
    let res = app.run(&mut terminal, events).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }

    Ok(())
}
