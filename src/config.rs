use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: Option<String>,
    #[serde(default = "default_theme")]
    pub theme: Theme,
    #[serde(default = "default_theme_name")]
    pub theme_name: String,
    #[serde(default)]
    pub neovim: NeovimConfig,
    #[serde(default)]
    pub vim_mode: bool,
    #[serde(default)]
    pub last_conversation_id: Option<String>,
    #[serde(default = "default_true")]
    pub notify_on_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    #[serde(default = "default_accent_color")]
    pub accent: String,
    #[serde(default = "default_user_color")]
    pub user_color: String,
    #[serde(default = "default_assistant_color")]
    pub assistant_color: String,
    #[serde(default = "default_border_color")]
    pub border_color: String,
    #[serde(default = "default_dim_color")]
    pub dim_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NeovimConfig {
    #[serde(default)]
    pub auto_connect: bool,
    #[serde(default)]
    pub socket_path: Option<String>,
    #[serde(default = "default_true")]
    pub send_code_blocks: bool,
}

fn default_provider() -> String { "anthropic".into() }
fn default_model() -> String { "claude-sonnet-4-20250514".into() }
fn default_max_tokens() -> u32 { 8192 }
fn default_temperature() -> f32 { 0.7 }
fn default_true() -> bool { true }
fn default_system_prompt() -> Option<String> {
    Some("You are a helpful AI assistant. When writing code, you are precise and produce clean, working code. You format responses using markdown. When asked to edit files or write code, use the available tools to read, write, and edit files directly. Be concise but thorough.".into())
}

fn default_theme_name() -> String { "tokyo-night".into() }
fn default_accent_color() -> String { "#7aa2f7".into() }
fn default_user_color() -> String { "#9ece6a".into() }
fn default_assistant_color() -> String { "#bb9af7".into() }
fn default_border_color() -> String { "#3b4261".into() }
fn default_dim_color() -> String { "#565f89".into() }

/// Resolved theme colors for use in the UI.
#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub accent: Color,
    pub user_label: Color,
    pub assistant_label: Color,
    pub border: Color,
    pub dim: Color,
    pub bg_dark: Color,
    pub fg: Color,
    pub warning: Color,
    pub success: Color,
}

/// Return the ThemeColors for a given theme name.
/// Falls back to tokyo-night for unknown names.
pub fn get_theme(name: &str) -> ThemeColors {
    match name {
        "catppuccin" => ThemeColors {
            accent: Color::Rgb(0x89, 0xb4, 0xfa),
            user_label: Color::Rgb(0xa6, 0xe3, 0xa1),
            assistant_label: Color::Rgb(0xcb, 0xa6, 0xf7),
            border: Color::Rgb(0x45, 0x47, 0x5a),
            dim: Color::Rgb(0x6c, 0x70, 0x86),
            bg_dark: Color::Rgb(0x1e, 0x1e, 0x2e),
            fg: Color::Rgb(0xcd, 0xd6, 0xf4),
            warning: Color::Rgb(0xf9, 0xe2, 0xaf),
            success: Color::Rgb(0xa6, 0xe3, 0xa1),
        },
        "gruvbox" => ThemeColors {
            accent: Color::Rgb(0x83, 0xa5, 0x98),
            user_label: Color::Rgb(0xb8, 0xbb, 0x26),
            assistant_label: Color::Rgb(0xd3, 0x86, 0x9b),
            border: Color::Rgb(0x50, 0x49, 0x45),
            dim: Color::Rgb(0x92, 0x83, 0x74),
            bg_dark: Color::Rgb(0x1d, 0x20, 0x21),
            fg: Color::Rgb(0xeb, 0xdb, 0xb2),
            warning: Color::Rgb(0xfa, 0xbd, 0x2f),
            success: Color::Rgb(0xb8, 0xbb, 0x26),
        },
        "dracula" => ThemeColors {
            accent: Color::Rgb(0x8b, 0xe9, 0xfd),
            user_label: Color::Rgb(0x50, 0xfa, 0x7b),
            assistant_label: Color::Rgb(0xbd, 0x93, 0xf9),
            border: Color::Rgb(0x44, 0x47, 0x5a),
            dim: Color::Rgb(0x62, 0x72, 0xa4),
            bg_dark: Color::Rgb(0x21, 0x22, 0x2c),
            fg: Color::Rgb(0xf8, 0xf8, 0xf2),
            warning: Color::Rgb(0xf1, 0xfa, 0x8c),
            success: Color::Rgb(0x50, 0xfa, 0x7b),
        },
        // tokyo-night (default)
        _ => ThemeColors {
            accent: Color::Rgb(0x7a, 0xa2, 0xf7),
            user_label: Color::Rgb(0x9e, 0xce, 0x6a),
            assistant_label: Color::Rgb(0xbb, 0x9a, 0xf7),
            border: Color::Rgb(0x3b, 0x42, 0x61),
            dim: Color::Rgb(0x56, 0x5f, 0x89),
            bg_dark: Color::Rgb(0x16, 0x16, 0x1e),
            fg: Color::Rgb(0xc0, 0xca, 0xf5),
            warning: Color::Rgb(0xe0, 0xaf, 0x68),
            success: Color::Rgb(0x9e, 0xce, 0x6a),
        },
    }
}

fn default_theme() -> Theme {
    Theme {
        accent: default_accent_color(),
        user_color: default_user_color(),
        assistant_color: default_assistant_color(),
        border_color: default_border_color(),
        dim_color: default_dim_color(),
    }
}

impl Default for Theme {
    fn default() -> Self { default_theme() }
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pro-chat")
            .join("config.toml")
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = Self::path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn api_key(&self) -> Option<&str> {
        match self.provider.as_str() {
            "anthropic" => self.anthropic_api_key.as_deref()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().as_deref().map(|_| unreachable!())),
            "openai" => self.openai_api_key.as_deref(),
            _ => None,
        }
    }

    pub fn api_key_from_env(&self) -> Option<String> {
        match self.provider.as_str() {
            "anthropic" => self.anthropic_api_key.clone()
                .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok()),
            "openai" => self.openai_api_key.clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok()),
            _ => None,
        }
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("pro-chat")
    }

    pub fn history_dir() -> PathBuf {
        Self::data_dir().join("conversations")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            anthropic_api_key: None,
            openai_api_key: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            system_prompt: default_system_prompt(),
            theme: default_theme(),
            theme_name: default_theme_name(),
            neovim: NeovimConfig::default(),
            vim_mode: false,
            last_conversation_id: None,
            notify_on_complete: true,
        }
    }
}
