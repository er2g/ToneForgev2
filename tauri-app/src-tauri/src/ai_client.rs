//! Multi-Provider AI Client
//!
//! Unified interface for multiple AI providers:
//! - OpenAI (GPT-4, GPT-3.5)
//! - Anthropic Claude (Sonnet, Opus, Haiku)
//! - Google Gemini (Pro, Flash)
//! - xAI Grok

use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone)]
pub enum AIProvider {
    OpenAI { api_key: String, model: String },
    Claude { api_key: String, model: String },
    Gemini { api_key: String, model: String },
    Grok { api_key: String, model: String },
}

impl AIProvider {
    /// Create OpenAI provider
    pub fn openai(api_key: String, model: String) -> Self {
        AIProvider::OpenAI { api_key, model }
    }

    /// Create Claude provider
    pub fn claude(api_key: String, model: String) -> Self {
        AIProvider::Claude { api_key, model }
    }

    /// Create Gemini provider
    pub fn gemini(api_key: String, model: String) -> Self {
        AIProvider::Gemini { api_key, model }
    }

    /// Create Grok provider
    pub fn grok(api_key: String, model: String) -> Self {
        AIProvider::Grok { api_key, model }
    }

    /// Get provider name
    pub fn name(&self) -> &str {
        match self {
            AIProvider::OpenAI { .. } => "OpenAI",
            AIProvider::Claude { .. } => "Claude",
            AIProvider::Gemini { .. } => "Gemini",
            AIProvider::Grok { .. } => "Grok",
        }
    }

    /// Get model name
    pub fn model_name(&self) -> &str {
        match self {
            AIProvider::OpenAI { model, .. } => model,
            AIProvider::Claude { model, .. } => model,
            AIProvider::Gemini { model, .. } => model,
            AIProvider::Grok { model, .. } => model,
        }
    }

    /// Generate completion with system prompt and user message
    pub async fn generate(
        &self,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, Box<dyn Error>> {
        match self {
            AIProvider::OpenAI { api_key, model } => {
                self.generate_openai(api_key, model, system_prompt, user_message)
                    .await
            }
            AIProvider::Claude { api_key, model } => {
                self.generate_claude(api_key, model, system_prompt, user_message)
                    .await
            }
            AIProvider::Gemini { api_key, model } => {
                self.generate_gemini(api_key, model, system_prompt, user_message)
                    .await
            }
            AIProvider::Grok { api_key, model } => {
                self.generate_grok(api_key, model, system_prompt, user_message)
                    .await
            }
        }
    }

    // ==================== OPENAI ====================

    async fn generate_openai(
        &self,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, Box<dyn Error>> {
        #[derive(Serialize)]
        struct OpenAIRequest {
            model: String,
            messages: Vec<OpenAIMessage>,
            temperature: f32,
        }

        #[derive(Serialize)]
        struct OpenAIMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct OpenAIResponse {
            choices: Vec<OpenAIChoice>,
        }

        #[derive(Deserialize)]
        struct OpenAIChoice {
            message: OpenAIMessage,
        }

        #[derive(Deserialize)]
        struct OpenAIMessage {
            content: String,
        }

        let client = reqwest::Client::new();

        let request = OpenAIRequest {
            model: model.to_string(),
            messages: vec![
                OpenAIMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                OpenAIMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            temperature: 0.7,
        };

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("OpenAI API error: {}", error_text).into());
        }

        let parsed: OpenAIResponse = response.json().await?;
        let content = parsed
            .choices
            .get(0)
            .map(|choice| choice.message.content.clone())
            .ok_or("No response from OpenAI")?;

        Ok(content.trim().to_string())
    }

    // ==================== CLAUDE ====================

    async fn generate_claude(
        &self,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, Box<dyn Error>> {
        #[derive(Serialize)]
        struct ClaudeRequest {
            model: String,
            max_tokens: u32,
            system: String,
            messages: Vec<ClaudeMessage>,
        }

        #[derive(Serialize, Deserialize)]
        struct ClaudeMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct ClaudeResponse {
            content: Vec<ClaudeContent>,
        }

        #[derive(Deserialize)]
        struct ClaudeContent {
            text: String,
        }

        let client = reqwest::Client::new();

        let request = ClaudeRequest {
            model: model.to_string(),
            max_tokens: 4096,
            system: system_prompt.to_string(),
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
        };

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Claude API error: {}", error_text).into());
        }

        let parsed: ClaudeResponse = response.json().await?;
        let content = parsed
            .content
            .get(0)
            .map(|c| c.text.clone())
            .ok_or("No response from Claude")?;

        Ok(content.trim().to_string())
    }

    // ==================== GEMINI ====================

    async fn generate_gemini(
        &self,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, Box<dyn Error>> {
        #[derive(Serialize)]
        struct GeminiRequest {
            contents: Vec<GeminiContent>,
            #[serde(rename = "systemInstruction")]
            system_instruction: GeminiSystemInstruction,
        }

        #[derive(Serialize)]
        struct GeminiSystemInstruction {
            parts: Vec<GeminiPart>,
        }

        #[derive(Serialize)]
        struct GeminiContent {
            parts: Vec<GeminiPart>,
        }

        #[derive(Serialize)]
        struct GeminiPart {
            text: String,
        }

        #[derive(Deserialize)]
        struct GeminiResponse {
            candidates: Vec<GeminiCandidate>,
        }

        #[derive(Deserialize)]
        struct GeminiCandidate {
            content: GeminiResponseContent,
        }

        #[derive(Deserialize)]
        struct GeminiResponseContent {
            parts: Vec<GeminiResponsePart>,
        }

        #[derive(Deserialize)]
        struct GeminiResponsePart {
            text: String,
        }

        let client = reqwest::Client::new();

        let request = GeminiRequest {
            system_instruction: GeminiSystemInstruction {
                parts: vec![GeminiPart {
                    text: system_prompt.to_string(),
                }],
            },
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: user_message.to_string(),
                }],
            }],
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini API error: {}", error_text).into());
        }

        let parsed: GeminiResponse = response.json().await?;
        let content = parsed
            .candidates
            .get(0)
            .and_then(|c| c.content.parts.get(0))
            .map(|p| p.text.clone())
            .ok_or("No response from Gemini")?;

        Ok(content.trim().to_string())
    }

    // ==================== GROK (xAI) ====================

    async fn generate_grok(
        &self,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, Box<dyn Error>> {
        #[derive(Serialize)]
        struct GrokRequest {
            model: String,
            messages: Vec<GrokMessage>,
        }

        #[derive(Serialize, Deserialize)]
        struct GrokMessage {
            role: String,
            content: String,
        }

        #[derive(Deserialize)]
        struct GrokResponse {
            choices: Vec<GrokChoice>,
        }

        #[derive(Deserialize)]
        struct GrokChoice {
            message: GrokMessage,
        }

        let client = reqwest::Client::new();

        let request = GrokRequest {
            model: model.to_string(),
            messages: vec![
                GrokMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                GrokMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
        };

        let response = client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Grok API error: {}", error_text).into());
        }

        let parsed: GrokResponse = response.json().await?;
        let content = parsed
            .choices
            .get(0)
            .map(|choice| choice.message.content.clone())
            .ok_or("No response from Grok")?;

        Ok(content.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let openai = AIProvider::openai("test-key".to_string(), "gpt-4".to_string());
        assert_eq!(openai.name(), "OpenAI");
        assert_eq!(openai.model_name(), "gpt-4");

        let claude = AIProvider::claude("test-key".to_string(), "claude-3-sonnet-20240229".to_string());
        assert_eq!(claude.name(), "Claude");

        let gemini = AIProvider::gemini("test-key".to_string(), "gemini-pro".to_string());
        assert_eq!(gemini.name(), "Gemini");

        let grok = AIProvider::grok("test-key".to_string(), "grok-beta".to_string());
        assert_eq!(grok.name(), "Grok");
    }
}
