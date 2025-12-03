//! Tone Research Layer
//!
//! This module implements the first AI layer that researches guitar/bass tones
//! from the internet before the main AI processes the request.
//!
//! When a user asks for a specific tone (e.g., "Chuck Schuldiner Symbolic tone"),
//! this layer will:
//! 1. Detect the tone request
//! 2. Search multiple sources (Equipboard, forums, YouTube, etc.)
//! 3. Extract detailed tone information (amp settings, effects, parameters)
//! 4. Format it for the main AI layer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

const CACHE_TTL_SECS: u64 = 7 * 24 * 60 * 60; // 7 days
const REQUEST_TIMEOUT_SECS: u64 = 5;
const MAX_SEARCH_RESULTS: usize = 5;

/// Represents a tone request detected from user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneRequest {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub song: Option<String>,
    pub genre: Option<String>,
    pub instrument: Option<String>, // guitar, bass, etc.
    pub raw_query: String,
}

/// Detailed tone information gathered from research
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneInfo {
    pub description: String,
    pub amp_settings: HashMap<String, String>,
    pub effects_chain: Vec<Effect>,
    pub equipment: Vec<String>,
    pub techniques: Vec<String>,
    pub sources: Vec<String>, // URLs where info was found
    pub confidence: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub name: String,
    pub effect_type: String, // distortion, delay, reverb, etc.
    pub parameters: HashMap<String, String>,
}

/// Cached research result with timestamp
#[derive(Debug, Clone)]
struct CachedResult {
    info: ToneInfo,
    timestamp: SystemTime,
}

/// Main tone researcher that coordinates internet research
pub struct ToneResearcher {
    http_client: reqwest::Client,
    cache: Arc<Mutex<HashMap<String, CachedResult>>>,
}

impl ToneResearcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .user_agent("ToneForge/1.0")
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client: client,
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Detect if a message contains a tone request
    pub fn detect_tone_request(&self, message: &str) -> Option<ToneRequest> {
        let msg_lower = message.to_lowercase();

        // Keywords that indicate a tone request
        let tone_keywords = [
            "tone", "sound", "tonu", "ses", "ayar", "settings",
            "amp", "pedal", "effect", "distortion", "reverb", "delay"
        ];

        let has_tone_keyword = tone_keywords.iter().any(|kw| msg_lower.contains(kw));

        if !has_tone_keyword {
            return None;
        }

        // Try to extract artist/album/song names
        // This is a simple heuristic - can be improved with NLP
        let words: Vec<&str> = message.split_whitespace().collect();

        Some(ToneRequest {
            artist: Self::extract_artist(&words),
            album: Self::extract_album(&words, message),
            song: Self::extract_song(&words),
            genre: Self::extract_genre(&msg_lower),
            instrument: Self::extract_instrument(&msg_lower),
            raw_query: message.to_string(),
        })
    }

    fn extract_artist(words: &[&str]) -> Option<String> {
        // Look for capitalized words that might be artist names
        let artist_words: Vec<String> = words
            .iter()
            .filter(|w| {
                w.len() > 1 &&
                w.chars().next().unwrap().is_uppercase() &&
                !w.to_lowercase().contains("tone") &&
                !w.to_lowercase().contains("sound")
            })
            .map(|w| w.to_string())
            .collect();

        if artist_words.is_empty() {
            None
        } else {
            Some(artist_words.join(" "))
        }
    }

    fn extract_album(words: &[&str], full_message: &str) -> Option<String> {
        // Look for album indicators
        let album_markers = ["album", "albüm", "from"];

        for (i, word) in words.iter().enumerate() {
            if album_markers.contains(&word.to_lowercase().as_str()) && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
        }

        // Look for quoted album names
        if let Some(start) = full_message.find('"') {
            if let Some(end) = full_message[start + 1..].find('"') {
                return Some(full_message[start + 1..start + 1 + end].to_string());
            }
        }

        None
    }

    fn extract_song(words: &[&str]) -> Option<String> {
        // Look for song indicators
        let song_markers = ["song", "şarkı", "track"];

        for (i, word) in words.iter().enumerate() {
            if song_markers.contains(&word.to_lowercase().as_str()) && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
        }

        None
    }

    fn extract_genre(msg_lower: &str) -> Option<String> {
        let genres = [
            "metal", "rock", "jazz", "blues", "punk", "grunge",
            "death metal", "black metal", "thrash metal", "progressive",
            "indie", "alternative", "classic rock"
        ];

        for genre in genres {
            if msg_lower.contains(genre) {
                return Some(genre.to_string());
            }
        }

        None
    }

    fn extract_instrument(msg_lower: &str) -> Option<String> {
        if msg_lower.contains("bass") || msg_lower.contains("bas") {
            Some("bass".to_string())
        } else if msg_lower.contains("guitar") || msg_lower.contains("gitar") {
            Some("guitar".to_string())
        } else {
            Some("guitar".to_string()) // Default to guitar
        }
    }

    /// Main research function that coordinates all sources
    pub async fn research_tone(&self, request: &ToneRequest) -> Result<ToneInfo, String> {
        // Check cache first
        let cache_key = self.make_cache_key(request);

        if let Some(cached) = self.get_from_cache(&cache_key) {
            println!("[ToneResearcher] Cache hit for: {}", cache_key);
            return Ok(cached);
        }

        println!("[ToneResearcher] Researching tone: {:?}", request);

        // Perform parallel searches across multiple sources
        let mut tone_info = ToneInfo {
            description: String::new(),
            amp_settings: HashMap::new(),
            effects_chain: Vec::new(),
            equipment: Vec::new(),
            techniques: Vec::new(),
            sources: Vec::new(),
            confidence: 0.0,
        };

        // Search different sources
        let equipboard_task = self.search_equipboard(request);
        let duckduckgo_task = self.search_web(request);
        let youtube_task = self.search_youtube_metadata(request);

        // Gather results
        if let Ok(equipboard_info) = equipboard_task.await {
            Self::merge_tone_info(&mut tone_info, equipboard_info);
        }

        if let Ok(web_info) = duckduckgo_task.await {
            Self::merge_tone_info(&mut tone_info, web_info);
        }

        if let Ok(youtube_info) = youtube_task.await {
            Self::merge_tone_info(&mut tone_info, youtube_info);
        }

        // Calculate confidence based on amount of information gathered
        tone_info.confidence = self.calculate_confidence(&tone_info);

        // Store in cache
        self.store_in_cache(&cache_key, tone_info.clone());

        Ok(tone_info)
    }

    async fn search_equipboard(&self, request: &ToneRequest) -> Result<ToneInfo, String> {
        if let Some(ref artist) = request.artist {
            let query = format!("equipboard {} guitar pedals amplifier", artist);
            let search_url = format!(
                "https://html.duckduckgo.com/html/?q={}",
                urlencoding::encode(&query)
            );

            let response = self.http_client
                .get(&search_url)
                .send()
                .await
                .map_err(|e| format!("Equipboard search failed: {}", e))?;

            let html = response.text().await.map_err(|e| e.to_string())?;

            // Parse equipment mentions from HTML
            let equipment = Self::parse_equipment_from_html(&html);

            Ok(ToneInfo {
                description: format!("Equipment used by {}", artist),
                amp_settings: HashMap::new(),
                effects_chain: Vec::new(),
                equipment,
                techniques: Vec::new(),
                sources: vec![search_url],
                confidence: 0.0,
            })
        } else {
            Err("No artist specified".to_string())
        }
    }

    async fn search_web(&self, request: &ToneRequest) -> Result<ToneInfo, String> {
        let query = self.build_search_query(request);
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(&query)
        );

        let response = self.http_client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| format!("Web search failed: {}", e))?;

        let html = response.text().await.map_err(|e| e.to_string())?;

        // Extract tone information from search results
        let effects = Self::parse_effects_from_html(&html);
        let amp_settings = Self::parse_amp_settings_from_html(&html);
        let techniques = Self::parse_techniques_from_html(&html);

        Ok(ToneInfo {
            description: format!("Web search results for: {}", query),
            amp_settings,
            effects_chain: effects,
            equipment: Vec::new(),
            techniques,
            sources: vec![search_url],
            confidence: 0.0,
        })
    }

    async fn search_youtube_metadata(&self, request: &ToneRequest) -> Result<ToneInfo, String> {
        let query = format!("{} tone tutorial settings", request.raw_query);
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q=site:youtube.com+{}",
            urlencoding::encode(&query)
        );

        let response = self.http_client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| format!("YouTube search failed: {}", e))?;

        let html = response.text().await.map_err(|e| e.to_string())?;

        // Extract video descriptions and settings
        let techniques = Self::parse_techniques_from_html(&html);

        Ok(ToneInfo {
            description: "YouTube tutorial findings".to_string(),
            amp_settings: HashMap::new(),
            effects_chain: Vec::new(),
            equipment: Vec::new(),
            techniques,
            sources: vec![search_url],
            confidence: 0.0,
        })
    }

    fn build_search_query(&self, request: &ToneRequest) -> String {
        let mut parts = Vec::new();

        if let Some(ref artist) = request.artist {
            parts.push(artist.clone());
        }
        if let Some(ref album) = request.album {
            parts.push(album.clone());
        }
        if let Some(ref song) = request.song {
            parts.push(song.clone());
        }

        parts.push("guitar tone settings".to_string());

        if let Some(ref genre) = request.genre {
            parts.push(genre.clone());
        }

        parts.join(" ")
    }

    // HTML parsing helpers
    fn parse_equipment_from_html(html: &str) -> Vec<String> {
        let mut equipment = Vec::new();

        // Look for common equipment brands and types
        let equipment_keywords = [
            "Marshall", "Fender", "Mesa Boogie", "Orange", "Vox", "Peavey",
            "Gibson", "Ibanez", "ESP", "PRS",
            "Boss", "MXR", "TC Electronic", "Strymon", "Electro-Harmonix",
            "Tube Screamer", "Big Muff", "Rat", "Blues Driver",
            "Les Paul", "Stratocaster", "Telecaster", "SG"
        ];

        for keyword in equipment_keywords {
            if html.to_lowercase().contains(&keyword.to_lowercase()) {
                equipment.push(keyword.to_string());
            }
        }

        equipment.sort();
        equipment.dedup();
        equipment
    }

    fn parse_effects_from_html(html: &str) -> Vec<Effect> {
        let mut effects = Vec::new();
        let html_lower = html.to_lowercase();

        // Common effects to look for
        let effect_patterns = [
            ("distortion", "Distortion"),
            ("overdrive", "Overdrive"),
            ("fuzz", "Fuzz"),
            ("delay", "Delay"),
            ("reverb", "Reverb"),
            ("chorus", "Chorus"),
            ("flanger", "Flanger"),
            ("phaser", "Phaser"),
            ("wah", "Wah"),
            ("compressor", "Compressor"),
            ("eq", "EQ"),
            ("boost", "Boost"),
        ];

        for (pattern, effect_type) in effect_patterns {
            if html_lower.contains(pattern) {
                effects.push(Effect {
                    name: effect_type.to_string(),
                    effect_type: effect_type.to_string(),
                    parameters: HashMap::new(),
                });
            }
        }

        effects
    }

    fn parse_amp_settings_from_html(html: &str) -> HashMap<String, String> {
        let mut settings = HashMap::new();
        let html_lower = html.to_lowercase();

        // Look for common amp settings mentions
        let setting_patterns = [
            ("gain", r"gain[:\s]+(\d+)"),
            ("bass", r"bass[:\s]+(\d+)"),
            ("mid", r"mid[:\s]+(\d+)"),
            ("treble", r"treble[:\s]+(\d+)"),
            ("presence", r"presence[:\s]+(\d+)"),
            ("volume", r"volume[:\s]+(\d+)"),
        ];

        for (param, _pattern) in setting_patterns {
            // Simple heuristic: if parameter is mentioned, note it
            if html_lower.contains(param) {
                settings.insert(
                    param.to_string(),
                    "See detailed description".to_string()
                );
            }
        }

        settings
    }

    fn parse_techniques_from_html(html: &str) -> Vec<String> {
        let mut techniques = Vec::new();
        let html_lower = html.to_lowercase();

        let technique_keywords = [
            "palm mute", "palm muting",
            "down picking", "alternate picking",
            "legato", "sweep picking",
            "tremolo picking", "vibrato",
            "pinch harmonic", "tapping",
            "drop tuning", "standard tuning",
            "low gain", "high gain",
            "scooped mids", "mid boost"
        ];

        for keyword in technique_keywords {
            if html_lower.contains(keyword) {
                techniques.push(keyword.to_string());
            }
        }

        techniques.sort();
        techniques.dedup();
        techniques
    }

    fn merge_tone_info(target: &mut ToneInfo, source: ToneInfo) {
        // Merge descriptions
        if !source.description.is_empty() {
            if target.description.is_empty() {
                target.description = source.description;
            } else {
                target.description.push_str("\n\n");
                target.description.push_str(&source.description);
            }
        }

        // Merge amp settings
        for (key, value) in source.amp_settings {
            target.amp_settings.entry(key).or_insert(value);
        }

        // Merge effects (avoid duplicates)
        for effect in source.effects_chain {
            if !target.effects_chain.iter().any(|e| e.name == effect.name) {
                target.effects_chain.push(effect);
            }
        }

        // Merge equipment
        for equip in source.equipment {
            if !target.equipment.contains(&equip) {
                target.equipment.push(equip);
            }
        }

        // Merge techniques
        for tech in source.techniques {
            if !target.techniques.contains(&tech) {
                target.techniques.push(tech);
            }
        }

        // Merge sources
        target.sources.extend(source.sources);
    }

    fn calculate_confidence(&self, info: &ToneInfo) -> f32 {
        let mut score: f32 = 0.0;

        // Score based on amount of information
        if !info.description.is_empty() { score += 0.2; }
        if !info.amp_settings.is_empty() { score += 0.2; }
        if !info.effects_chain.is_empty() { score += 0.2; }
        if !info.equipment.is_empty() { score += 0.2; }
        if !info.techniques.is_empty() { score += 0.1; }
        if !info.sources.is_empty() { score += 0.1; }

        score.min(1.0)
    }

    // Cache management
    fn make_cache_key(&self, request: &ToneRequest) -> String {
        format!(
            "{}_{}_{}_{}",
            request.artist.as_deref().unwrap_or(""),
            request.album.as_deref().unwrap_or(""),
            request.song.as_deref().unwrap_or(""),
            request.genre.as_deref().unwrap_or("")
        )
        .to_lowercase()
        .replace(' ', "_")
    }

    fn get_from_cache(&self, key: &str) -> Option<ToneInfo> {
        let cache = self.cache.lock().unwrap();

        if let Some(cached) = cache.get(key) {
            // Check if cache entry is still valid
            if let Ok(elapsed) = cached.timestamp.elapsed() {
                if elapsed.as_secs() < CACHE_TTL_SECS {
                    return Some(cached.info.clone());
                }
            }
        }

        None
    }

    fn store_in_cache(&self, key: &str, info: ToneInfo) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(
            key.to_string(),
            CachedResult {
                info,
                timestamp: SystemTime::now(),
            },
        );
    }

    /// Format tone info into a context string for the main AI
    pub fn format_for_ai(&self, info: &ToneInfo) -> String {
        let mut output = String::new();

        output.push_str("=== TONE RESEARCH RESULTS ===\n\n");

        if !info.description.is_empty() {
            output.push_str(&format!("Description:\n{}\n\n", info.description));
        }

        if !info.equipment.is_empty() {
            output.push_str("Equipment:\n");
            for item in &info.equipment {
                output.push_str(&format!("  - {}\n", item));
            }
            output.push('\n');
        }

        if !info.amp_settings.is_empty() {
            output.push_str("Amp Settings:\n");
            for (param, value) in &info.amp_settings {
                output.push_str(&format!("  - {}: {}\n", param, value));
            }
            output.push('\n');
        }

        if !info.effects_chain.is_empty() {
            output.push_str("Effects Chain:\n");
            for effect in &info.effects_chain {
                output.push_str(&format!("  - {} ({})\n", effect.name, effect.effect_type));
                if !effect.parameters.is_empty() {
                    for (param, value) in &effect.parameters {
                        output.push_str(&format!("    - {}: {}\n", param, value));
                    }
                }
            }
            output.push('\n');
        }

        if !info.techniques.is_empty() {
            output.push_str("Playing Techniques:\n");
            for tech in &info.techniques {
                output.push_str(&format!("  - {}\n", tech));
            }
            output.push('\n');
        }

        output.push_str(&format!("Confidence: {:.0}%\n", info.confidence * 100.0));

        if !info.sources.is_empty() {
            output.push_str("\nSources:\n");
            for source in &info.sources {
                output.push_str(&format!("  - {}\n", source));
            }
        }

        output.push_str("\n=== END RESEARCH RESULTS ===\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_tone_request() {
        let researcher = ToneResearcher::new();

        // Test with artist and album
        let request = researcher.detect_tone_request("Chuck Schuldiner Symbolic tone");
        assert!(request.is_some());
        let req = request.unwrap();
        assert!(req.artist.is_some());
        assert!(req.raw_query.contains("Symbolic"));

        // Test without tone keyword
        let request = researcher.detect_tone_request("Hello world");
        assert!(request.is_none());
    }

    #[test]
    fn test_genre_extraction() {
        let msg = "I want a death metal tone";
        let genre = ToneResearcher::extract_genre(&msg.to_lowercase());
        assert_eq!(genre, Some("death metal".to_string()));
    }

    #[test]
    fn test_instrument_extraction() {
        let msg = "bass guitar settings";
        let instrument = ToneResearcher::extract_instrument(&msg.to_lowercase());
        assert_eq!(instrument, Some("bass".to_string()));
    }
}
