//! Researcher Mode - Tone Research and Discussion
//!
//! This mode is for tone research and discussion without REAPER connection.
//! Users can:
//! - Search the tone encyclopedia
//! - Ask about tone history and equipment
//! - Discuss tones with AI
//! - Get recommendations
//!
//! NO REAPER connection or modifications!

use crate::ai_client::AIProvider;
use crate::conversation::{Message, MessageMetadata, MessageRole};
use crate::tone_encyclopedia::ToneEncyclopedia;
use serde::{Deserialize, Serialize};

const CONTEXT_MESSAGE_LIMIT: usize = 10;

/// Researcher mode handler
pub struct ResearcherMode {
    encyclopedia: ToneEncyclopedia,
    ai_provider: AIProvider,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResearcherResponse {
    pub content: String,
    pub encyclopedia_matches: Vec<EncyclopediaMatch>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncyclopediaMatch {
    pub id: String,
    pub artist: String,
    pub album: Option<String>,
    pub song: Option<String>,
    pub description: String,
    pub score: f32,
}

impl ResearcherMode {
    /// Create new researcher mode handler
    pub fn new(encyclopedia: ToneEncyclopedia, ai_provider: AIProvider) -> Self {
        Self {
            encyclopedia,
            ai_provider,
        }
    }

    /// Process a research request
    pub async fn process_message(
        &self,
        user_message: &str,
        conversation_history: &[&Message],
    ) -> Result<ResearcherResponse, String> {
        println!("[RESEARCHER MODE] Processing: {}", user_message);

        // Step 1: Search encyclopedia
        let search_results = self.encyclopedia.search(user_message, 5);

        let mut encyclopedia_matches = Vec::new();
        let mut encyclopedia_context = String::new();

        if !search_results.is_empty() {
            encyclopedia_context.push_str("=== RELEVANT TONES FROM ENCYCLOPEDIA ===\n\n");

            for result in &search_results {
                encyclopedia_matches.push(EncyclopediaMatch {
                    id: result.tone.id.clone(),
                    artist: result.tone.artist.clone(),
                    album: result.tone.album.clone(),
                    song: result.tone.song.clone(),
                    description: result.tone.description.clone(),
                    score: result.score,
                });

                encyclopedia_context.push_str(&format!(
                    "**{} - {}**\n",
                    result.tone.artist,
                    result.tone.album.as_deref().unwrap_or("Unknown Album")
                ));

                if let Some(ref song) = result.tone.song {
                    encyclopedia_context.push_str(&format!("Song: {}\n", song));
                }

                encyclopedia_context.push_str(&format!("Description: {}\n", result.tone.description));

                if let Some(ref genre) = result.tone.genre {
                    encyclopedia_context.push_str(&format!("Genre: {}\n", genre));
                }

                // Equipment details
                if let Some(ref amp) = result.tone.equipment.amp {
                    encyclopedia_context.push_str(&format!("Amp: {}\n", amp));
                }

                if let Some(ref guitar) = result.tone.equipment.guitar {
                    encyclopedia_context.push_str(&format!("Guitar: {}\n", guitar));
                }

                if !result.tone.equipment.pedals.is_empty() {
                    encyclopedia_context.push_str(&format!(
                        "Pedals: {}\n",
                        result.tone.equipment.pedals.join(", ")
                    ));
                }

                // Basic parameter overview
                if !result.tone.parameters.amp.is_empty() {
                    encyclopedia_context.push_str("Amp Settings:\n");
                    for (key, value) in &result.tone.parameters.amp {
                        encyclopedia_context.push_str(&format!("  - {}: {:.2}\n", key, value));
                    }
                }

                encyclopedia_context.push_str(&format!("Match Score: {:.0}%\n\n", result.score * 100.0));
            }

            encyclopedia_context.push_str("=== END ENCYCLOPEDIA RESULTS ===\n\n");

            println!(
                "[RESEARCHER MODE] Found {} encyclopedia matches",
                encyclopedia_matches.len()
            );
        }

        // Step 2: Build AI prompt
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(user_message, conversation_history, &encyclopedia_context);

        // Step 3: Get AI response
        let ai_response = self
            .ai_provider
            .generate(&system_prompt, &user_prompt)
            .await
            .map_err(|e| format!("AI error: {}", e))?;

        // Step 4: Extract suggestions (if any)
        let suggestions = self.extract_suggestions(&ai_response);

        Ok(ResearcherResponse {
            content: ai_response,
            encyclopedia_matches,
            suggestions,
        })
    }

    fn build_system_prompt(&self) -> String {
        format!(
            r#"You are a guitar/bass tone research specialist and music historian.

Your role is to help users research and understand legendary guitar and bass tones.

CAPABILITIES:
- You have access to a tone encyclopedia with {} tones
- You can discuss tone history, equipment, and techniques
- You can recommend tones based on user preferences
- You can explain how to achieve specific tones

LIMITATIONS:
- You CANNOT modify REAPER or apply changes (you're in Researcher mode)
- You can only discuss and research tones

STYLE:
- Be enthusiastic about music and gear
- Share historical context and interesting facts
- Explain technical details when relevant
- Make recommendations based on user needs

When encyclopedia results are provided, reference them in your response.
If no matches are found, use your knowledge to help the user.

IMPORTANT: Do NOT suggest applying changes to REAPER. This mode is for research only.
If the user wants to apply a tone, suggest they switch to "Act" mode."#,
            self.encyclopedia.count()
        )
    }

    fn build_user_prompt(
        &self,
        user_message: &str,
        conversation_history: &[&Message],
        encyclopedia_context: &str,
    ) -> String {
        let mut prompt = String::new();

        // Add conversation context
        if !conversation_history.is_empty() {
            prompt.push_str("=== CONVERSATION HISTORY ===\n");
            for msg in conversation_history.iter().rev().take(CONTEXT_MESSAGE_LIMIT).rev() {
                let role = match msg.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                };
                prompt.push_str(&format!("{}: {}\n", role, msg.content));
            }
            prompt.push_str("\n");
        }

        // Add encyclopedia context
        if !encyclopedia_context.is_empty() {
            prompt.push_str(encyclopedia_context);
        }

        // Add user message
        prompt.push_str(&format!("=== CURRENT REQUEST ===\n{}\n", user_message));

        prompt
    }

    fn extract_suggestions(&self, ai_response: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Simple heuristic: look for lines starting with "- " or numbered lists
        for line in ai_response.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                suggestions.push(trimmed[2..].to_string());
            } else if trimmed.len() > 3 && trimmed.chars().nth(0).unwrap_or(' ').is_numeric() && trimmed.contains(". ") {
                if let Some(content) = trimmed.split(". ").nth(1) {
                    suggestions.push(content.to_string());
                }
            }
        }

        suggestions
    }

    /// Get encyclopedia statistics for context
    pub fn get_encyclopedia_stats(&self) -> EncyclopediaStats {
        EncyclopediaStats {
            total_tones: self.encyclopedia.count(),
            genres: self.encyclopedia.get_all_genres(),
            artists: self.encyclopedia.get_all_artists(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EncyclopediaStats {
    pub total_tones: usize,
    pub genres: Vec<String>,
    pub artists: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tone_encyclopedia::{Equipment, ToneEntry, ToneParameters};
    use std::collections::HashMap;

    #[test]
    fn test_researcher_mode_creation() {
        let encyclopedia = ToneEncyclopedia::new();
        let provider = crate::ai_client::AIProvider::grok("test".to_string(), "test".to_string());
        let researcher = ResearcherMode::new(encyclopedia, provider);

        let stats = researcher.get_encyclopedia_stats();
        assert_eq!(stats.total_tones, 0);
    }

    #[test]
    fn test_suggestion_extraction() {
        let encyclopedia = ToneEncyclopedia::new();
        let provider = crate::ai_client::AIProvider::grok("test".to_string(), "test".to_string());
        let researcher = ResearcherMode::new(encyclopedia, provider);

        let response = "Here are some suggestions:\n- Use a Tube Screamer\n- Try scooped mids\n- Add some reverb";
        let suggestions = researcher.extract_suggestions(response);

        assert_eq!(suggestions.len(), 3);
        assert!(suggestions[0].contains("Tube Screamer"));
    }
}
