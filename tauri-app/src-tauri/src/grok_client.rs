use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

use crate::ConversationEntry;

#[derive(Debug, Clone)]
pub struct GrokClient {
    api_key: String,
    client: reqwest::Client,
    model: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<GrokMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatChoice {
    message: GrokMessage,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

impl GrokClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            model,
        }
    }

    pub async fn generate_chat(
        &self,
        system_prompt: &str,
        history: &[ConversationEntry],
        user_prompt: &str,
    ) -> Result<String, Box<dyn Error>> {
        let mut messages: Vec<GrokMessage> = Vec::new();

        if !system_prompt.trim().is_empty() {
            messages.push(GrokMessage {
                role: "system".into(),
                content: system_prompt.to_string(),
            });
        }

        for entry in history {
            messages.push(GrokMessage {
                role: entry.role.clone(),
                content: entry.content.clone(),
            });
        }

        messages.push(GrokMessage {
            role: "user".into(),
            content: user_prompt.to_string(),
        });

        let request_body = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.25),
            max_output_tokens: Some(2048),
            stream: false,
        };

        let response = self
            .client
            .post("https://api.x.ai/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Grok API error: {}", error_text).into());
        }

        let grok_response: ChatResponse = response.json().await?;

        let first = grok_response
            .choices
            .get(0)
            .map(|choice| choice.message.content.clone())
            .ok_or("No response from Grok")?;

        Ok(first.trim().to_string())
    }

    pub async fn generate(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        self.generate_chat("", &[], prompt).await
    }
}
