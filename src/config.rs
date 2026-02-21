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
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_theme")]
    pub theme: Theme,
    #[serde(default)]
    pub neovim: NeovimConfig,
    #[serde(default)]
    pub vim_mode: bool,
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

fn default_accent_color() -> String { "#7aa2f7".into() }
fn default_user_color() -> String { "#9ece6a".into() }
fn default_assistant_color() -> String { "#bb9af7".into() }
fn default_border_color() -> String { "#3b4261".into() }
fn default_dim_color() -> String { "#565f89".into() }

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
            system_prompt: None,
            theme: default_theme(),
            neovim: NeovimConfig::default(),
            vim_mode: false,
        }
    }
}
