//! Tone Encyclopedia System
//!
//! This module manages the tone encyclopedia - a database of thousands of guitar/bass tones
//! from famous albums and artists. The encyclopedia is stored in JSON format and provides
//! fuzzy search capabilities to find matching tones.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Main tone encyclopedia structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneEncyclopedia {
    pub version: String,
    pub tones: Vec<ToneEntry>,
}

/// A single tone entry in the encyclopedia
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneEntry {
    pub id: String,
    pub artist: String,
    pub album: Option<String>,
    pub song: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub instrument: String, // "guitar", "bass"
    pub description: String,

    #[serde(default)]
    pub equipment: Equipment,

    pub parameters: ToneParameters,

    #[serde(default)]
    pub techniques: Vec<String>,

    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Equipment {
    pub guitar: Option<String>,
    pub amp: Option<String>,
    pub cabinet: Option<String>,
    pub pedals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneParameters {
    #[serde(default)]
    pub amp: HashMap<String, f64>,

    #[serde(default)]
    pub eq: HashMap<String, f64>, // Frequency -> dB

    #[serde(default)]
    pub effects: Vec<EffectParameters>,

    #[serde(default)]
    pub reverb: HashMap<String, f64>,

    #[serde(default)]
    pub delay: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectParameters {
    pub effect_type: String, // "noise_gate", "overdrive", "distortion", etc.
    pub parameters: HashMap<String, f64>,
}

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub tone: ToneEntry,
    pub score: f32, // 0.0 - 1.0
    pub matched_fields: Vec<String>,
}

impl ToneEncyclopedia {
    /// Create a new empty encyclopedia
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            tones: Vec::new(),
        }
    }

    /// Load encyclopedia from JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    /// Save encyclopedia to JSON file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))?;

        fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))
    }

    /// Add a tone entry
    pub fn add_tone(&mut self, tone: ToneEntry) {
        self.tones.push(tone);
    }

    /// Search for tones matching a query
    /// Returns results sorted by relevance (highest first)
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<SearchResult> = Vec::new();

        for tone in &self.tones {
            let (score, matched_fields) = self.calculate_relevance(tone, &query_lower);

            if score > 0.0 {
                results.push(SearchResult {
                    tone: tone.clone(),
                    score,
                    matched_fields,
                });
            }
        }

        // Sort by score (descending)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(limit);

        results
    }

    /// Calculate relevance score for a tone against a query
    fn calculate_relevance(&self, tone: &ToneEntry, query_lower: &str) -> (f32, Vec<String>) {
        let mut score: f32 = 0.0;
        let mut matched_fields = Vec::new();

        // Artist match (highest weight)
        if self.fuzzy_match(&tone.artist.to_lowercase(), query_lower) {
            score += 10.0;
            matched_fields.push(format!("artist: {}", tone.artist));
        }

        // Album match
        if let Some(ref album) = tone.album {
            if self.fuzzy_match(&album.to_lowercase(), query_lower) {
                score += 8.0;
                matched_fields.push(format!("album: {}", album));
            }
        }

        // Song match
        if let Some(ref song) = tone.song {
            if self.fuzzy_match(&song.to_lowercase(), query_lower) {
                score += 7.0;
                matched_fields.push(format!("song: {}", song));
            }
        }

        // Genre match
        if let Some(ref genre) = tone.genre {
            if query_lower.contains(&genre.to_lowercase()) || genre.to_lowercase().contains(query_lower) {
                score += 5.0;
                matched_fields.push(format!("genre: {}", genre));
            }
        }

        // Equipment match
        if let Some(ref amp) = tone.equipment.amp {
            if self.fuzzy_match(&amp.to_lowercase(), query_lower) {
                score += 4.0;
                matched_fields.push(format!("amp: {}", amp));
            }
        }

        if let Some(ref guitar) = tone.equipment.guitar {
            if self.fuzzy_match(&guitar.to_lowercase(), query_lower) {
                score += 3.0;
                matched_fields.push(format!("guitar: {}", guitar));
            }
        }

        // Tags match
        for tag in &tone.tags {
            if query_lower.contains(&tag.to_lowercase()) || tag.to_lowercase().contains(query_lower) {
                score += 2.0;
                matched_fields.push(format!("tag: {}", tag));
            }
        }

        // Description match
        if self.fuzzy_match(&tone.description.to_lowercase(), query_lower) {
            score += 1.0;
            matched_fields.push("description".to_string());
        }

        // Normalize score to 0-1 range
        let normalized_score = (score / 20.0).min(1.0);

        (normalized_score, matched_fields)
    }

    /// Simple fuzzy matching - checks if strings contain each other or share significant substrings
    fn fuzzy_match(&self, text: &str, query: &str) -> bool {
        // Direct substring match
        if text.contains(query) || query.contains(text) {
            return true;
        }

        // Word-level matching
        let text_words: Vec<&str> = text.split_whitespace().collect();
        let query_words: Vec<&str> = query.split_whitespace().collect();

        for query_word in &query_words {
            for text_word in &text_words {
                if text_word.contains(query_word) || query_word.contains(text_word) {
                    if query_word.len() > 3 && text_word.len() > 3 {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get tone by ID
    pub fn get_by_id(&self, id: &str) -> Option<&ToneEntry> {
        self.tones.iter().find(|t| t.id == id)
    }

    /// Get total number of tones
    pub fn count(&self) -> usize {
        self.tones.len()
    }

    /// Get all genres in the encyclopedia
    pub fn get_all_genres(&self) -> Vec<String> {
        let mut genres: Vec<String> = self.tones
            .iter()
            .filter_map(|t| t.genre.clone())
            .collect();
        genres.sort();
        genres.dedup();
        genres
    }

    /// Get all artists in the encyclopedia
    pub fn get_all_artists(&self) -> Vec<String> {
        let mut artists: Vec<String> = self.tones
            .iter()
            .map(|t| t.artist.clone())
            .collect();
        artists.sort();
        artists.dedup();
        artists
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encyclopedia_creation() {
        let encyclopedia = ToneEncyclopedia::new();
        assert_eq!(encyclopedia.version, "1.0");
        assert_eq!(encyclopedia.count(), 0);
    }

    #[test]
    fn test_add_and_search() {
        let mut encyclopedia = ToneEncyclopedia::new();

        let tone = ToneEntry {
            id: "metallica_master_battery".to_string(),
            artist: "Metallica".to_string(),
            album: Some("Master of Puppets".to_string()),
            song: Some("Battery".to_string()),
            year: Some(1986),
            genre: Some("Thrash Metal".to_string()),
            instrument: "guitar".to_string(),
            description: "Aggressive rhythm tone".to_string(),
            equipment: Equipment::default(),
            parameters: ToneParameters {
                amp: HashMap::new(),
                eq: HashMap::new(),
                effects: Vec::new(),
                reverb: HashMap::new(),
                delay: HashMap::new(),
            },
            techniques: Vec::new(),
            tags: vec!["aggressive".to_string(), "scooped".to_string()],
        };

        encyclopedia.add_tone(tone);

        // Search for "Metallica"
        let results = encyclopedia.search("Metallica", 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.0);

        // Search for "Battery"
        let results = encyclopedia.search("Battery", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_fuzzy_matching() {
        let encyclopedia = ToneEncyclopedia::new();

        assert!(encyclopedia.fuzzy_match("metallica", "metal"));
        assert!(encyclopedia.fuzzy_match("master of puppets", "master"));
        assert!(encyclopedia.fuzzy_match("gibson explorer", "gibson"));
    }
}
