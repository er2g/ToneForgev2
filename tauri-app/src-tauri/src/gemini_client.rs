use reqwest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct GeminiClient {
    api_key: String,
    client: reqwest::Client,
    model: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Candidate {
    content: Content,
}

impl GeminiClient {
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
        history: &[crate::ConversationEntry],
        user_prompt: &str,
    ) -> Result<String, Box<dyn Error>> {
        let mut contents: Vec<Content> = Vec::new();

        for entry in history {
            contents.push(Content {
                role: Some(entry.role.clone()),
                parts: vec![Part {
                    text: entry.content.clone(),
                }],
            });
        }

        contents.push(Content {
            role: Some("user".into()),
            parts: vec![Part {
                text: user_prompt.to_string(),
            }],
        });

        let request_body = GeminiRequest {
            contents,
            system_instruction: Some(Content {
                role: Some("system".into()),
                parts: vec![Part {
                    text: system_prompt.to_string(),
                }],
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self.client.post(&url).json(&request_body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini API error: {}", error_text).into());
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let first = gemini_response
            .candidates
            .get(0)
            .and_then(|candidate| candidate.content.parts.get(0))
            .map(|part| part.text.clone())
            .ok_or("No response from Gemini")?;

        Ok(first.trim().to_string())
    }

    pub async fn generate(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        let request_body = GeminiRequest {
            contents: vec![Content {
                role: Some("user".into()),
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            system_instruction: None,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = self.client.post(&url).json(&request_body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini API error: {}", error_text).into());
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let first = gemini_response
            .candidates
            .get(0)
            .and_then(|candidate| candidate.content.parts.get(0))
            .map(|part| part.text.clone())
            .ok_or("No response from Gemini")?;

        Ok(first.trim().to_string())
    }
}
