use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;

use crate::event::Event;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
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

            // Process SSE lines
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        let _ = tx.send(Event::ApiDone);
                        return Ok(());
                    }

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                        // Handle content_block_delta
                        if event["type"] == "content_block_delta" {
                            if let Some(text) = event["delta"]["text"].as_str() {
                                let _ = tx.send(Event::ApiChunk(text.to_string()));
                            }
                        }
                        // Handle message_stop
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

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
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

                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
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
}
