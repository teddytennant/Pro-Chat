use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<SavedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: "New conversation".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            messages: Vec::new(),
        }
    }

    fn path(&self) -> PathBuf {
        Config::history_dir().join(format!("{}.json", self.id))
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Config::history_dir();
        std::fs::create_dir_all(&dir)?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(self.path(), content)?;
        Ok(())
    }

    pub fn load(id: &str) -> anyhow::Result<Self> {
        let path = Config::history_dir().join(format!("{id}.json"));
        let content = std::fs::read_to_string(path)?;
        let conv: Conversation = serde_json::from_str(&content)?;
        Ok(conv)
    }

    pub fn list_all() -> anyhow::Result<Vec<Conversation>> {
        let dir = Config::history_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut convs = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(conv) = serde_json::from_str::<Conversation>(&content) {
                        convs.push(conv);
                    }
                }
            }
        }
        convs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(convs)
    }

    /// Returns the most recently updated conversation (by updated_at timestamp).
    pub fn latest() -> anyhow::Result<Option<Conversation>> {
        let convs = Self::list_all()?;
        Ok(convs.into_iter().next())
    }

    pub fn delete(id: &str) -> anyhow::Result<()> {
        let path = Config::history_dir().join(format!("{id}.json"));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.messages.push(SavedMessage {
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();

        // Auto-title from first user message
        if self.title == "New conversation" {
            if let Some(first_user) = self.messages.iter().find(|m| m.role == "user") {
                let title: String = first_user.content.chars().take(60).collect();
                self.title = if title.len() < first_user.content.len() {
                    format!("{title}...")
                } else {
                    title
                };
            }
        }
    }
}
