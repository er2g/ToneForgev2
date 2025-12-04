use crate::ConversationEntry;
use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct XaiClient {
    api_key: String,
    client: reqwest::Client,
    model: String,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

impl XaiClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
            model,
            base_url: "https://api.x.ai/v1".to_string(),
        }
    }

    pub async fn generate_chat(
        &self,
        system_prompt: &str,
        history: &[ConversationEntry],
        user_prompt: &str,
    ) -> Result<String, Box<dyn Error>> {
        let mut messages: Vec<ChatMessage> = Vec::new();
        messages.push(ChatMessage {
            role: "system".into(),
            content: system_prompt.to_string(),
        });

        for entry in history {
            messages.push(ChatMessage {
                role: entry.role.clone(),
                content: entry.content.clone(),
            });
        }

        messages.push(ChatMessage {
            role: "user".into(),
            content: user_prompt.to_string(),
        });

        let request_body = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("xAI API error: {}", error_text).into());
        }

        let parsed: ChatCompletionResponse = response.json().await?;
        let content = parsed
            .choices
            .get(0)
            .map(|choice| choice.message.content.clone())
            .ok_or("No response from xAI")?;

        Ok(content.trim().to_string())
    }

    pub async fn generate(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        self.generate_chat(
            "You are Grok responding concisely to a direct prompt.",
            &[],
            prompt,
        )
        .await
    }
}
