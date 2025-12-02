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
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
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

    pub async fn generate(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        let request_body = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
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
