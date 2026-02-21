use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::event::Event;
use crate::tools;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

/// Anthropic API supports both simple string content and structured content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<Value>),
}

impl MessageContent {
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Blocks(blocks) => {
                blocks.iter()
                    .filter_map(|b| {
                        if b["type"] == "text" {
                            b["text"].as_str().map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }
        }
    }
}

pub struct ApiClient {
    client: Client,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Stream an Anthropic API call (text-only, no tools).
    pub async fn stream_anthropic(
        &self,
        api_key: &str,
        model: &str,
        messages: &[Message],
        system_prompt: Option<&str>,
        max_tokens: u32,
        temperature: f32,
        tx: mpsc::UnboundedSender<Event>,
    ) -> anyhow::Result<()> {
        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": true,
            "messages": messages,
        });

        if let Some(sys) = system_prompt {
            body["system"] = json!(sys);
        }

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let _ = tx.send(Event::ApiError(format!("API error {status}: {text}")));
            return Ok(());
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        let _ = tx.send(Event::ApiDone);
                        return Ok(());
                    }

                    if let Ok(event) = serde_json::from_str::<Value>(data) {
                        if event["type"] == "content_block_delta" {
                            if let Some(text) = event["delta"]["text"].as_str() {
                                let _ = tx.send(Event::ApiChunk(text.to_string()));
                            }
                        }
                        if event["type"] == "message_stop" {
                            let _ = tx.send(Event::ApiDone);
                            return Ok(());
                        }
                    }
                }
            }
        }

        let _ = tx.send(Event::ApiDone);
        Ok(())
    }

    /// Non-streaming Anthropic call with tool definitions.
    /// Returns the full response body if it contains tool_use blocks,
    /// otherwise streams the text content via events.
    pub async fn call_anthropic_with_tools(
        &self,
        api_key: &str,
        model: &str,
        messages: &[Message],
        system_prompt: Option<&str>,
        max_tokens: u32,
        temperature: f32,
        tx: mpsc::UnboundedSender<Event>,
    ) -> anyhow::Result<()> {
        let tool_defs = tools::format_tool_definitions();

        let mut body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "messages": messages,
            "tools": tool_defs,
        });

        if let Some(sys) = system_prompt {
            body["system"] = json!(sys);
        }

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let _ = tx.send(Event::ApiError(format!("API error {status}: {text}")));
            return Ok(());
        }

        let body_text = response.text().await?;
        let response_json: Value = serde_json::from_str(&body_text)?;

        // Check if response contains tool_use blocks
        let has_tool_use = response_json["content"]
            .as_array()
            .map(|arr| arr.iter().any(|b| b["type"] == "tool_use"))
            .unwrap_or(false);

        if has_tool_use {
            // Extract any text content first and send it
            if let Some(content) = response_json["content"].as_array() {
                for block in content {
                    if block["type"] == "text" {
                        if let Some(text) = block["text"].as_str() {
                            let _ = tx.send(Event::ApiChunk(text.to_string()));
                        }
                    }
                }
            }
            // Send the full response for tool processing
            let _ = tx.send(Event::ToolUseRequest(body_text));
        } else {
            // No tools - just send the text content
            if let Some(content) = response_json["content"].as_array() {
                for block in content {
                    if block["type"] == "text" {
                        if let Some(text) = block["text"].as_str() {
                            let _ = tx.send(Event::ApiChunk(text.to_string()));
                        }
                    }
                }
            }
            let _ = tx.send(Event::ApiDone);
        }

        Ok(())
    }

    /// Stream an OpenAI-compatible API call (works for OpenAI, OpenRouter, xAI, etc.).
    pub async fn stream_openai_compatible(
        &self,
        api_key: &str,
        model: &str,
        messages: &[Message],
        system_prompt: Option<&str>,
        max_tokens: u32,
        temperature: f32,
        tx: mpsc::UnboundedSender<Event>,
        base_url: &str,
        extra_headers: &[(&str, &str)],
    ) -> anyhow::Result<()> {
        let mut msgs = Vec::new();
        if let Some(sys) = system_prompt {
            msgs.push(json!({"role": "system", "content": sys}));
        }
        for msg in messages {
            msgs.push(json!({"role": msg.role, "content": msg.content}));
        }

        let body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": true,
            "messages": msgs,
        });

        let mut req = self.client
            .post(base_url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("content-type", "application/json");

        for (key, value) in extra_headers {
            req = req.header(*key, *value);
        }

        let response = req.json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let _ = tx.send(Event::ApiError(format!("API error {status}: {text}")));
            return Ok(());
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        let _ = tx.send(Event::ApiDone);
                        return Ok(());
                    }

                    if let Ok(event) = serde_json::from_str::<Value>(data) {
                        if let Some(content) = event["choices"][0]["delta"]["content"].as_str() {
                            let _ = tx.send(Event::ApiChunk(content.to_string()));
                        }
                    }
                }
            }
        }

        let _ = tx.send(Event::ApiDone);
        Ok(())
    }

    pub async fn stream_openai(
        &self,
        api_key: &str,
        model: &str,
        messages: &[Message],
        system_prompt: Option<&str>,
        max_tokens: u32,
        temperature: f32,
        tx: mpsc::UnboundedSender<Event>,
    ) -> anyhow::Result<()> {
        self.stream_openai_compatible(
            api_key, model, messages, system_prompt,
            max_tokens, temperature, tx,
            "https://api.openai.com/v1/chat/completions",
            &[],
        ).await
    }
}
