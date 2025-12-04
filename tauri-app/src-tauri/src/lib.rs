mod ai_engine;
mod ai_engine_tests;
mod audio;
mod dsp;
mod errors;
mod gemini_client;
mod reaper_client;
mod secure_storage;
mod tone_researcher;
mod undo_redo;

use audio::analyzer::{analyze_spectrum, AnalysisConfig};
use audio::loader::{load_audio_file, resample_audio};
use audio::matcher::{match_profiles, MatchConfig as EqMatchConfig, MatchResult as EqMatchResult};
use audio::profile::{extract_eq_profile, EQProfile};
use errors::{ErrorResponse, ToneForgeError};
use gemini_client::GeminiClient;
use reaper_client::ReaperClient;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tone_researcher::ToneResearcher;
use undo_redo::{UndoAction, UndoActionSummary, UndoManager, UndoState};

const MAX_HISTORY: usize = 40;
const PROMPT_HISTORY_LIMIT: usize = 12;
const MAX_RECENT_TONES: usize = 20;

/// Recent tone entry for quick access
#[derive(Clone, Serialize, Deserialize)]
struct RecentTone {
    id: String,
    query: String,
    summary: String,
    timestamp: u64,
    track: i32,
    changes_count: usize,
}

// Helper to handle mutex poisoning gracefully
fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        eprintln!("⚠️  Mutex was poisoned, recovering...");
        poisoned.into_inner()
    })
}

const SYSTEM_PROMPT: &str = r#"
You are an autonomous tone engineer for guitar/bass production. You see the complete plugin chain state and must make intelligent modifications based on user requests.

=== TWO-LAYER AI SYSTEM ===

You are the SECOND AI layer in a two-layer system:

🔍 FIRST LAYER (Tone Research AI):
- When users request specific tones (e.g., "Chuck Schuldiner Symbolic tone", "Metallica Master of Puppets sound")
- Automatically searches the internet (Equipboard, forums, YouTube, etc.)
- Gathers detailed information: equipment, amp settings, effects chain, techniques
- Provides you with a "TONE RESEARCH RESULTS" section if available

🎛️ SECOND LAYER (You - Tone Implementation AI):
- You receive the research results from the first AI layer
- Your job is to IMPLEMENT those findings using available plugins
- Match the described tone as closely as possible with current plugin parameters
- If research results are available, USE THEM as your primary reference

=== UNDERSTANDING THE DATA ===

Each parameter has:
- value: 0.0-1.0 normalized (what you SET)
- display: real-world value with units ("-6.2 dB", "432 Hz")
- format_hint: type (decibel/frequency/percentage/time/raw)

Each FX has:
- enabled: whether plugin is active
- params: all parameters with current state

=== YOUR CAPABILITIES ===

1. MODIFY: set_param, toggle_fx, load_plugin
2. RESEARCH: web_search (use when you need additional info beyond the first AI layer)
3. REASON: Think through the problem before acting

=== CRITICAL PRINCIPLES ===

🔴 HIERARCHICAL VALIDATION:
Before touching ANY control, verify the hierarchy:
Plugin enabled? → Section/pedal enabled? → Parameter accessible?

If something is disabled at ANY level, enable it FIRST, then proceed.
Example: Changing "Overdrive Gain" requires:
  1. Plugin enabled ✓
  2. "Overdrive On" parameter = active ✓
  3. Then modify gain

🔴 POST-ACTION VERIFICATION:
After you return actions, the system will re-fetch state and show you the results.
Your actions will be applied, then you'll see if they worked.
Plan accordingly - if something might need follow-up, mention it.

🔴 RESEARCH, DON'T GUESS:
Don't know Metallica tone settings? web_search it.
Unsure which plugin for jazz? web_search it.
You have internet access - use it.

=== RESPONSE FORMAT ===

Return JSON with this structure (but express yourself naturally):
{
  "summary": "Your brief explanation of what you're doing and why",
  "changes_table": [
    {"plugin": "...", "parameter": "...", "old_value": "...", "new_value": "...", "reason": "..."}
  ],
  "actions": [
    {"type": "set_param", "track": 0, "fx_index": 0, "param_index": 1, "value": 0.75, "reason": "..."},
    {"type": "web_search", "query": "Neural DSP Gojira Metallica settings", "reason": "..."},
    {"type": "toggle_fx", "track": 0, "fx_index": 0, "enabled": true, "reason": "..."}
  ]
}

changes_table is for USER visibility (show display values).
actions is for EXECUTION (use normalized 0-1 values).

=== THINK FREELY ===

- Use your judgment on parameter ranges
- Calculate display values based on parameter type
- Explain your reasoning naturally
- If uncertain, research or ask for clarification
- Multi-step changes are fine (enable, then modify, then verify)

=== WHAT YOU SEE VS WHAT USER SEES ===

You see: Raw JSON snapshot, all parameters, technical data
User sees: Your summary + changes_table in a nice format
User does NOT see: The actions array, technical logs, or internal reasoning

Be technical in actions, human in summary/changes_table.
"#;

#[derive(Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: u64,
}

#[derive(Clone)]
enum AIProvider {
    Gemini(GeminiClient),
}

impl AIProvider {
    async fn generate(&self, prompt: &str) -> Result<String, Box<dyn Error>> {
        match self {
            AIProvider::Gemini(client) => client.generate(prompt).await,
        }
    }
}

struct AppState {
    reaper: Mutex<ReaperClient>,
    ai_provider: Mutex<Option<AIProvider>>,
    chat_history: Mutex<Vec<ChatMessage>>,
    http_client: reqwest::Client, // Reusable HTTP client (no mutex needed - Clone is cheap)
    tone_researcher: ToneResearcher, // First AI layer for internet research
    undo_manager: Mutex<UndoManager>, // Undo/Redo system
    recent_tones: Mutex<Vec<RecentTone>>, // Recent tone history
}

#[derive(Clone, Serialize, Deserialize)]
struct FxParamState {
    index: i32,
    name: String,
    value: f64,
    display: String,
    unit: String,
    format_hint: String,
}

#[derive(Clone, Serialize)]
struct FxState {
    index: i32,
    name: String,
    enabled: bool,
    params: Vec<FxParamState>,
}

#[derive(Clone, Serialize)]
struct TrackSnapshot {
    index: i32,
    name: String,
    fx: Vec<FxState>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PresetParamState {
    index: i32,
    name: String,
    value: f64,
}

#[derive(Clone, Serialize, Deserialize)]
struct PresetFxState {
    index: i32,
    name: String,
    params: Vec<PresetParamState>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PresetTrackState {
    index: i32,
    name: String,
    fx: Vec<PresetFxState>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PresetFile {
    name: String,
    created_at: u64,
    project_path: Option<String>,
    tracks: Vec<PresetTrackState>,
}

#[derive(Serialize)]
struct PromptPayload {
    selected_track: i32,
    tracks: Vec<TrackSnapshot>,
    recent_messages: Vec<ConversationEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    research_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_instructions: Option<String>,
}

#[derive(Serialize)]
struct ConversationEntry {
    role: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChangeEntry {
    plugin: String,
    parameter: String,
    old_value: String,
    new_value: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponse {
    summary: String,
    changes_table: Vec<ChangeEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    engine_report: Option<String>,
    #[serde(default)]
    action_log: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AIPlan {
    summary: String,
    #[serde(default)]
    changes_table: Vec<ChangeEntry>,
    #[serde(default)]
    actions: Vec<PlannedAction>,
}

#[derive(Default)]
struct EngineDiagnostics {
    conflicts: Vec<String>,
    safety_warnings: Vec<String>,
    suggestions: Vec<String>,
    optimizations: Vec<String>,
    preflight: Vec<String>,
    focus_areas: Vec<String>,
}

impl EngineDiagnostics {
    fn record_conflicts(&mut self, conflicts: Vec<String>) {
        self.conflicts = conflicts;
    }

    fn push_safety_warning(&mut self, warning: String) {
        self.safety_warnings.push(warning);
    }

    fn push_suggestion(&mut self, suggestion: String) {
        self.suggestions.push(suggestion);
    }

    fn push_optimization(&mut self, note: String) {
        self.optimizations.push(note);
    }

    fn push_preflight(&mut self, note: String) {
        self.preflight.push(note);
    }

    fn push_focus(&mut self, focus: String) {
        self.focus_areas.push(focus);
    }

    fn is_empty(&self) -> bool {
        self.conflicts.is_empty()
            && self.safety_warnings.is_empty()
            && self.suggestions.is_empty()
            && self.optimizations.is_empty()
            && self.preflight.is_empty()
            && self.focus_areas.is_empty()
    }

    fn to_report(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut sections = Vec::new();

        if !self.optimizations.is_empty() {
            sections.push("Optimizations:".to_string());
            sections.extend(self.optimizations.iter().map(|note| format!("- {}", note)));
        }

        if !self.preflight.is_empty() {
            sections.push("Preflight readiness:".to_string());
            sections.extend(self.preflight.iter().map(|note| format!("- {}", note)));
        }

        if !self.conflicts.is_empty() {
            sections.push("Conflicts detected:".to_string());
            sections.extend(
                self.conflicts
                    .iter()
                    .map(|conflict| format!("- {}", conflict)),
            );
        }

        if !self.safety_warnings.is_empty() {
            sections.push("Safety warnings:".to_string());
            sections.extend(
                self.safety_warnings
                    .iter()
                    .map(|warn| format!("- {}", warn)),
            );
        }

        if !self.suggestions.is_empty() {
            sections.push("Tone compensations:".to_string());
            sections.extend(self.suggestions.iter().map(|note| format!("- {}", note)));
        }

        if !self.focus_areas.is_empty() {
            sections.push("Action focus:".to_string());
            sections.extend(self.focus_areas.iter().map(|area| format!("- {}", area)));
        }

        Some(sections.join("\n"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum PlannedAction {
    #[serde(rename = "set_param")]
    SetParam {
        track: i32,
        fx_index: i32,
        param_index: i32,
        value: f64,
        reason: Option<String>,
    },
    #[serde(rename = "toggle_fx")]
    ToggleFx {
        track: i32,
        fx_index: i32,
        enabled: bool,
        reason: Option<String>,
    },
    #[serde(rename = "load_plugin")]
    LoadPlugin {
        track: i32,
        plugin_name: String,
        position: Option<i32>,
        reason: Option<String>,
    },
    #[serde(rename = "web_search")]
    WebSearch {
        query: String,
        reason: Option<String>,
    },
    #[serde(rename = "noop")]
    Noop { reason: Option<String> },
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn push_history(history: &mut Vec<ChatMessage>, message: ChatMessage) {
    history.push(message);
    if history.len() > MAX_HISTORY {
        let overflow = history.len() - MAX_HISTORY;
        history.drain(0..overflow);
    }
}

fn conversation_for_prompt(history: &[ChatMessage]) -> Vec<ConversationEntry> {
    history
        .iter()
        .rev()
        .take(PROMPT_HISTORY_LIMIT)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|msg| ConversationEntry {
            role: msg.role,
            content: msg.content,
        })
        .collect()
}

async fn collect_track_snapshots(reaper: &ReaperClient) -> Result<Vec<TrackSnapshot>, String> {
    let overview = reaper.get_tracks().await.map_err(|e| e.to_string())?;
    let mut tracks = Vec::new();
    for track in overview.tracks.iter() {
        let mut fx_states = Vec::new();
        for fx in &track.fx_list {
            let snapshot = reaper
                .get_fx_params(track.index, fx.index)
                .await
                .map_err(|e| e.to_string())?;
            let params = snapshot
                .params
                .into_iter()
                .map(|entry| FxParamState {
                    index: entry.index,
                    name: entry.name,
                    value: entry.value,
                    display: entry.display,
                    unit: entry.unit,
                    format_hint: entry.format_hint,
                })
                .collect();
            fx_states.push(FxState {
                index: fx.index,
                name: fx.name.clone(),
                enabled: fx.enabled,
                params,
            });
        }
        tracks.push(TrackSnapshot {
            index: track.index,
            name: track.name.clone(),
            fx: fx_states,
        });
    }
    Ok(tracks)
}

fn build_prompt(payload: &PromptPayload) -> Result<String, String> {
    let context = serde_json::to_string_pretty(payload).map_err(|e| e.to_string())?;
    let track_hint = format!(
        "ACTIVE TARGET TRACK: {} (override by setting 'track' explicitly if you need another track)",
        payload.selected_track
    );

    let mut context_sections = vec![track_hint];

    // Add optional user instructions to steer the AI
    if let Some(ref instructions) = payload.custom_instructions {
        let trimmed = instructions.trim();
        if !trimmed.is_empty() {
            context_sections.push(format!("CUSTOM INSTRUCTIONS FROM USER:\n{}", trimmed));
        }
    }

    // Add research context if available (from first AI layer)
    if let Some(ref research) = payload.research_context {
        context_sections.push(research.clone());
    }

    Ok(format!(
        "{SYSTEM_PROMPT}\n\n{}\n\n=== SNAPSHOT START ===\n{}\n=== SNAPSHOT END ===",
        context_sections.join("\n\n"),
        context
    ))
}

fn extract_json_block(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        let without_ticks = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```JSON")
            .trim_start_matches("```")
            .trim();
        return without_ticks.trim_end_matches("```").trim().to_string();
    }
    if trimmed.starts_with('{') {
        return trimmed.to_string();
    }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }
    trimmed.to_string()
}

fn parse_plan(raw: &str) -> Result<AIPlan, String> {
    if let Ok(plan) = serde_json::from_str::<AIPlan>(raw) {
        return Ok(plan);
    }
    let candidate = extract_json_block(raw);
    serde_json::from_str(&candidate).map_err(|e| format!("Failed to parse AI JSON: {}", e))
}

fn normalize_action_track(
    action: PlannedAction,
    selected_track: i32,
    available_tracks: &HashSet<i32>,
) -> PlannedAction {
    let map_track = |track: i32| -> i32 {
        if available_tracks.contains(&track) {
            track
        } else {
            selected_track
        }
    };

    match action {
        PlannedAction::SetParam {
            track,
            fx_index,
            param_index,
            value,
            reason,
        } => PlannedAction::SetParam {
            track: map_track(track),
            fx_index,
            param_index,
            value,
            reason,
        },
        PlannedAction::ToggleFx {
            track,
            fx_index,
            enabled,
            reason,
        } => PlannedAction::ToggleFx {
            track: map_track(track),
            fx_index,
            enabled,
            reason,
        },
        PlannedAction::LoadPlugin {
            track,
            plugin_name,
            position,
            reason,
        } => PlannedAction::LoadPlugin {
            track: map_track(track),
            plugin_name,
            position,
            reason,
        },
        other => other,
    }
}

fn find_track<'a>(tracks: &'a [TrackSnapshot], track_idx: i32) -> Option<&'a TrackSnapshot> {
    tracks.iter().find(|t| t.index == track_idx)
}

fn find_fx<'a>(track: &'a TrackSnapshot, fx_idx: i32) -> Option<&'a FxState> {
    track.fx.iter().find(|fx| fx.index == fx_idx)
}

fn find_param<'a>(fx: &'a FxState, param_idx: i32) -> Option<&'a FxParamState> {
    fx.params.iter().find(|p| p.index == param_idx)
}

fn category_label(category: ai_engine::ParameterCategory) -> &'static str {
    match category {
        ai_engine::ParameterCategory::Distortion => "Distortion",
        ai_engine::ParameterCategory::EQ => "EQ",
        ai_engine::ParameterCategory::Dynamics => "Dynamics",
        ai_engine::ParameterCategory::Modulation => "Modulation",
        ai_engine::ParameterCategory::Delay => "Delay",
        ai_engine::ParameterCategory::Reverb => "Reverb",
        ai_engine::ParameterCategory::Filter => "Filter",
        ai_engine::ParameterCategory::Volume => "Volume",
        ai_engine::ParameterCategory::Toggle => "Toggle",
        ai_engine::ParameterCategory::Unknown => "Unknown",
    }
}

async fn perform_web_search(client: &reqwest::Client, query: &str) -> Result<String, String> {
    let search_url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let response = client
        .get(&search_url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| format!("Web search failed: {}", e))?;

    let html = response.text().await.map_err(|e| e.to_string())?;

    // Simple extraction: get first 500 chars of HTML as context
    // In production, you'd parse this properly or use a search API
    let preview = html.chars().take(1000).collect::<String>();
    Ok(format!(
        "Search results preview for '{}': {}",
        query, preview
    ))
}

fn build_action_reason(action: &ai_engine::ActionPlan, tracks: &[TrackSnapshot]) -> String {
    if let Some(track_state) = find_track(tracks, action.track) {
        if let Some(fx_state) = find_fx(track_state, action.fx_index) {
            if let Some(param_state) = find_param(fx_state, action.param_index) {
                return format!(
                    "Set '{}' on '{}' to {:.3} (was {:.3})",
                    param_state.name, fx_state.name, action.value, param_state.value
                );
            }
        }
    }

    "Set parameter to new value".into()
}

async fn apply_actions(
    reaper: &ReaperClient,
    http_client: &reqwest::Client,
    tracks: &[TrackSnapshot],
    actions: &[PlannedAction],
) -> Result<Vec<String>, String> {
    let mut logs = Vec::new();
    for action in actions {
        match action {
            PlannedAction::SetParam {
                track,
                fx_index,
                param_index,
                value,
                reason,
            } => {
                if let Some(track_state) = find_track(tracks, *track) {
                    if let Some(fx_state) = find_fx(track_state, *fx_index) {
                        // HIERARCHICAL VALIDATION: Check if FX is enabled
                        if !fx_state.enabled {
                            logs.push(format!(
                                "⚠️  Plugin '{}' is DISABLED. Enabling it first...",
                                fx_state.name
                            ));
                            reaper
                                .set_fx_enabled(*track, *fx_index, true)
                                .await
                                .map_err(|e| e.to_string())?;
                            logs.push(format!("✓ Enabled '{}'", fx_state.name));
                        }

                        if let Some(param_state) = find_param(fx_state, *param_index) {
                            let old_value = param_state.display.clone();

                            reaper
                                .set_param(*track, *fx_index, &param_state.name, *value)
                                .await
                                .map_err(|e| e.to_string())?;

                            logs.push(format!(
                                "✓ {} :: {} -> {} = {:.1}% ({})",
                                track_state.name,
                                fx_state.name,
                                param_state.name,
                                value * 100.0,
                                reason.clone().unwrap_or_else(|| "no reason".into())
                            ));

                            // POST-ACTION VERIFICATION: Re-fetch the parameter to confirm
                            match reaper.get_fx_params(*track, *fx_index).await {
                                Ok(updated_snapshot) => {
                                    if let Some(updated_param) = updated_snapshot
                                        .params
                                        .iter()
                                        .find(|p| p.index == *param_index)
                                    {
                                        logs.push(format!(
                                            "  ↳ Verified: {} → {} (was: {})",
                                            param_state.name, updated_param.display, old_value
                                        ));
                                    }
                                }
                                Err(e) => {
                                    logs.push(format!("  ⚠️  Could not verify change: {}", e));
                                }
                            }
                        } else {
                            logs.push(format!(
                                "⚠️  Skipped set_param: param {} not found on {}",
                                param_index, fx_state.name
                            ));
                        }
                    } else {
                        logs.push(format!(
                            "⚠️  Skipped set_param: fx {} not found on track {}",
                            fx_index, track_state.name
                        ));
                    }
                } else {
                    logs.push(format!("⚠️  Skipped set_param: track {} missing", track));
                }
            }
            PlannedAction::ToggleFx {
                track,
                fx_index,
                enabled,
                reason,
            } => {
                reaper
                    .set_fx_enabled(*track, *fx_index, *enabled)
                    .await
                    .map_err(|e| e.to_string())?;
                logs.push(format!(
                    "✓ Track {} FX {} toggled to {} ({})",
                    track,
                    fx_index,
                    enabled,
                    reason.clone().unwrap_or_else(|| "no reason".into())
                ));

                // POST-ACTION VERIFICATION: Re-fetch tracks to confirm
                match reaper.get_tracks().await {
                    Ok(overview) => {
                        if let Some(t) = overview.tracks.iter().find(|t| t.index == *track) {
                            if let Some(fx) = t.fx_list.iter().find(|f| f.index == *fx_index) {
                                logs.push(format!(
                                    "  ↳ Verified: '{}' is now {}",
                                    fx.name,
                                    if fx.enabled { "ENABLED" } else { "DISABLED" }
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        logs.push(format!("  ⚠️  Could not verify toggle: {}", e));
                    }
                }
            }
            PlannedAction::LoadPlugin {
                track,
                plugin_name,
                position: _,
                reason,
            } => {
                let slot = reaper
                    .add_plugin(*track, plugin_name)
                    .await
                    .map_err(|e| e.to_string())?;
                logs.push(format!(
                    "✓ Loaded '{}' on track {} slot {} ({})",
                    plugin_name,
                    track,
                    slot,
                    reason.clone().unwrap_or_else(|| "no reason".into())
                ));

                // POST-ACTION VERIFICATION: Check if plugin loaded
                match reaper.get_tracks().await {
                    Ok(overview) => {
                        if let Some(t) = overview.tracks.iter().find(|t| t.index == *track) {
                            if let Some(fx) = t.fx_list.iter().find(|f| f.index == slot) {
                                logs.push(format!(
                                    "  ↳ Verified: '{}' loaded at slot {} (enabled: {})",
                                    fx.name, slot, fx.enabled
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        logs.push(format!("  ⚠️  Could not verify load: {}", e));
                    }
                }
            }
            PlannedAction::WebSearch { query, reason } => {
                logs.push(format!(
                    "🔍 Researching: '{}' ({})",
                    query,
                    reason.clone().unwrap_or_else(|| "gathering info".into())
                ));
                match perform_web_search(http_client, query).await {
                    Ok(result) => {
                        logs.push(format!(
                            "  ✓ Search completed: {}",
                            &result[..200.min(result.len())]
                        ));
                    }
                    Err(e) => {
                        logs.push(format!("  ⚠️  Search failed: {}", e));
                    }
                }
            }
            PlannedAction::Noop { reason } => {
                logs.push(format!(
                    "ℹ️  No action: {}",
                    reason.clone().unwrap_or_else(|| "no changes needed".into())
                ));
            }
        }
    }
    Ok(logs)
}

#[tauri::command]
async fn load_reference_audio(path: String) -> Result<EQProfile, String> {
    let audio = load_audio_file(&path).map_err(|e| format!("Load error: {}", e))?;
    let target_rate = 48_000;
    let samples = if audio.sample_rate != target_rate {
        resample_audio(&audio.samples, audio.sample_rate, target_rate)
            .map_err(|e| format!("Resample error: {}", e))?
    } else {
        audio.samples
    };

    let config = AnalysisConfig::default();
    let spectrum = analyze_spectrum(&samples, target_rate, &config);
    Ok(extract_eq_profile(&spectrum, &config))
}

#[tauri::command]
async fn load_input_audio(path: String) -> Result<EQProfile, String> {
    load_reference_audio(path).await
}

#[tauri::command]
async fn calculate_eq_match(
    reference: EQProfile,
    input: EQProfile,
    config: EqMatchConfig,
) -> Result<EqMatchResult, String> {
    Ok(match_profiles(&reference, &input, &config))
}

#[tauri::command]
async fn export_eq_settings(result: EqMatchResult, format: String) -> Result<String, String> {
    match format.as_str() {
        "reaper" => export_as_reaper_preset(&result.correction_profile),
        "json" => {
            serde_json::to_string_pretty(&result.correction_profile).map_err(|e| e.to_string())
        }
        "txt" => export_as_text(&result.correction_profile),
        _ => Err("Unknown format".to_string()),
    }
}

fn export_as_reaper_preset(profile: &EQProfile) -> Result<String, String> {
    let mut output = String::from("<FXCHAIN\n");
    output.push_str("WNDRECT 0 0 0 0\n");
    output.push_str("SHOW 0\n");
    output.push_str("LASTSEL 0\n");
    output.push_str("DOCKED 0\n");
    output.push_str("<VST \"VST: ReaEQ (Cockos)\" ReaEQ 0 \"\" 1919247729\n");

    for (i, band) in profile.bands.iter().enumerate().take(10) {
        let base_param = i * 5;
        output.push_str(&format!("  {} 1.0\n", base_param));

        let freq_norm =
            (band.frequency.log2() - 20.0f32.log2()) / (20_000.0f32.log2() - 20.0f32.log2());
        output.push_str(&format!("  {} {}\n", base_param + 1, freq_norm));

        let gain_norm = (band.gain_db + 18.0) / 36.0;
        output.push_str(&format!("  {} {}\n", base_param + 2, gain_norm));

        output.push_str(&format!("  {} 0.5\n", base_param + 3));
        output.push_str(&format!("  {} 0.4\n", base_param + 4));
    }

    output.push_str(">\n");
    output.push_str("FLOATPOS 0 0 0 0\n");
    output.push_str("FXID {GUID}\n");
    output.push_str("WAK 0 0\n");
    output.push_str(">\n");

    Ok(output)
}

fn export_as_text(profile: &EQProfile) -> Result<String, String> {
    let mut output = String::from("EQ Settings:\n\n");
    for band in &profile.bands {
        output.push_str(&format!(
            "{:>6} Hz: {:>+6.2} dB (Q: {:.2})\n",
            band.frequency as i32,
            band.gain_db,
            calculate_q_from_bandwidth(band.frequency, band.bandwidth)
        ));
    }
    Ok(output)
}

fn calculate_q_from_bandwidth(center_freq: f32, bandwidth: f32) -> f32 {
    center_freq / bandwidth
}

#[tauri::command]
async fn check_reaper_connection(state: State<'_, AppState>) -> Result<bool, String> {
    let reaper = { lock_or_recover(&state.reaper).clone() };
    reaper.ping().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn configure_ai_provider(
    provider: String,
    model: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let provider_key = provider.to_lowercase();
    let ai_provider = match provider_key.as_str() {
        "gemini" => AIProvider::Gemini(GeminiClient::new(api_key, model.clone())),
        _ => return Err(format!("Unsupported provider: {}", provider)),
    };

    {
        let mut guard = lock_or_recover(&state.ai_provider);
        *guard = Some(ai_provider);
    }

    {
        let mut history = lock_or_recover(&state.chat_history);
        history.clear();
    }

    Ok(format!("{} configured (model: {})", provider, model))
}

#[tauri::command]
fn get_chat_history(state: State<'_, AppState>) -> Result<String, String> {
    let history = lock_or_recover(&state.chat_history);
    serde_json::to_string(&*history).map_err(|e| e.to_string())
}

#[tauri::command]
async fn process_chat_message(
    message: String,
    track: Option<i32>,
    custom_instructions: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ai_provider = {
        let guard = lock_or_recover(&state.ai_provider);
        guard
            .clone()
            .ok_or_else(|| "AI provider is not configured".to_string())?
    };

    let track_idx = track.unwrap_or(0).max(0);

    {
        let mut history = lock_or_recover(&state.chat_history);
        push_history(
            &mut history,
            ChatMessage {
                role: "user".into(),
                content: message.clone(),
                timestamp: current_timestamp(),
            },
        );
    }

    let reaper = { lock_or_recover(&state.reaper).clone() };
    let tracks_snapshot = collect_track_snapshots(&reaper).await?;

    let history_snapshot = {
        let history = lock_or_recover(&state.chat_history);
        conversation_for_prompt(&history)
    };

    // ========== TONE RESEARCH LAYER (First AI Layer) ==========
    // Detect if user is requesting a specific tone and research it from the internet
    let research_context =
        if let Some(tone_request) = state.tone_researcher.detect_tone_request(&message) {
            println!("[TONE RESEARCH] Detected tone request: {:?}", tone_request);

            match state.tone_researcher.research_tone(&tone_request).await {
                Ok(tone_info) => {
                    println!(
                        "[TONE RESEARCH] Research successful! Confidence: {:.0}%",
                        tone_info.confidence * 100.0
                    );
                    let formatted = state.tone_researcher.format_for_ai(&tone_info);
                    Some(formatted)
                }
                Err(e) => {
                    println!("[TONE RESEARCH] Research failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

    let payload = PromptPayload {
        selected_track: track_idx,
        tracks: tracks_snapshot.clone(),
        recent_messages: history_snapshot,
        research_context,
        custom_instructions,
    };

    let prompt = build_prompt(&payload)?;
    let ai_text = ai_provider
        .generate(&prompt)
        .await
        .map_err(|e| e.to_string())?;
    let mut plan = parse_plan(&ai_text)?;

    let mut diagnostics = EngineDiagnostics::default();

    let available_tracks: HashSet<i32> = tracks_snapshot.iter().map(|t| t.index).collect();
    plan.actions = plan
        .actions
        .into_iter()
        .map(|action| normalize_action_track(action, track_idx, &available_tracks))
        .collect();

    // ========== AI ENGINE: PROFESSIONAL OPTIMIZATIONS ==========
    println!(
        "[AI ENGINE] Processing {} total actions...",
        plan.actions.len()
    );

    // Separate SetParam actions for optimization, keep others as-is
    let mut set_param_actions: Vec<ai_engine::ActionPlan> = Vec::new();
    let mut other_actions: Vec<PlannedAction> = Vec::new();

    for action in plan.actions.iter() {
        match action {
            PlannedAction::SetParam {
                track,
                fx_index,
                param_index,
                value,
                reason,
            } => {
                set_param_actions.push(ai_engine::ActionPlan {
                    track: *track,
                    fx_index: *fx_index,
                    param_index: *param_index,
                    value: *value,
                    reason: reason.clone().unwrap_or_default(),
                });
            }
            _ => {
                // Keep ToggleFx, LoadPlugin, WebSearch, Noop as-is
                other_actions.push(action.clone());
            }
        }
    }

    println!(
        "[AI ENGINE] Split: {} SetParam actions, {} other actions",
        set_param_actions.len(),
        other_actions.len()
    );

    // 1. CONFLICT DETECTION (only for SetParam)
    let conflicts = ai_engine::ActionOptimizer::detect_conflicts(&set_param_actions);
    for conflict in &conflicts {
        println!("[AI ENGINE] ⚠️  {}", conflict);
    }
    diagnostics.record_conflicts(conflicts);

    // 2. ACTION DEDUPLICATION & OPTIMIZATION (only for SetParam)
    let deduplicated = ai_engine::ActionOptimizer::deduplicate(set_param_actions);
    if !deduplicated.is_empty() {
        diagnostics.push_optimization(format!(
            "SetParam actions optimized: {} → {} (deduplicated)",
            plan.actions
                .iter()
                .filter(|a| matches!(a, PlannedAction::SetParam { .. }))
                .count(),
            deduplicated.len()
        ));
    }
    println!(
        "[AI ENGINE] Deduplicated SetParam: {} → {} actions",
        plan.actions
            .iter()
            .filter(|a| matches!(a, PlannedAction::SetParam { .. }))
            .count(),
        deduplicated.len()
    );

    // 3. SAFETY VALIDATION (only for SetParam)
    let mut safety_warnings = Vec::new();
    for action in &deduplicated {
        // Find parameter name from snapshot
        if let Some(track_state) = find_track(&tracks_snapshot, action.track) {
            if let Some(fx_state) = find_fx(track_state, action.fx_index) {
                if let Some(param_state) = find_param(fx_state, action.param_index) {
                    let (clamped_value, warning) =
                        ai_engine::SafetyValidator::validate_value(&param_state.name, action.value);

                    if let Some(warn) = warning {
                        let formatted = format!(
                            "{} :: {} -> {}: {}",
                            fx_state.name, param_state.name, clamped_value, warn
                        );
                        diagnostics.push_safety_warning(formatted.clone());
                        safety_warnings.push(formatted);
                    }

                    // 4. SEMANTIC ANALYSIS & RELATIONSHIP SUGGESTIONS
                    let category = ai_engine::SemanticAnalyzer::categorize(&param_state.name);
                    println!(
                        "[AI ENGINE] Parameter '{}' categorized as {:?}",
                        param_state.name, category
                    );

                    let suggestions = ai_engine::RelationshipEngine::suggest_compensations(
                        &param_state.name,
                        param_state.value,
                        action.value,
                    );

                    for (param, delta, reason) in suggestions {
                        let suggestion = format!("Adjust '{}' by {:.2} ({})", param, delta, reason);
                        println!("[AI ENGINE] 💡 Suggestion: {}", suggestion);
                        diagnostics.push_suggestion(suggestion);
                    }
                } else {
                    diagnostics.push_safety_warning(format!(
                        "Param {} not found on track {} fx {} — cannot validate value.",
                        action.param_index, action.track, action.fx_index
                    ));
                }
            } else {
                diagnostics.push_safety_warning(format!(
                    "FX {} not available on track {} — skipping parameter edits.",
                    action.fx_index, action.track
                ));
            }
        } else {
            diagnostics.push_safety_warning(format!(
                "Track {} missing for planned action (fx {}, param {}).",
                action.track, action.fx_index, action.param_index
            ));
        }
    }

    for warning in &safety_warnings {
        println!("[AI ENGINE] 🛡️  Safety: {}", warning);
    }

    // Track explicit enable intents to avoid double toggles
    let planned_enables: HashSet<(i32, i32)> = other_actions
        .iter()
        .filter_map(|action| match action {
            PlannedAction::ToggleFx {
                track,
                fx_index,
                enabled: true,
                ..
            } => Some((*track, *fx_index)),
            _ => None,
        })
        .collect();

    // 5. PREFLIGHT: auto-enable FX that will be tweaked but are currently bypassed
    let mut auto_enable_actions = Vec::new();
    let mut auto_enabled_fx: HashSet<(i32, i32)> = HashSet::new();
    for action in &deduplicated {
        if planned_enables.contains(&(action.track, action.fx_index)) {
            continue;
        }

        if let Some(track_state) = find_track(&tracks_snapshot, action.track) {
            if let Some(fx_state) = find_fx(track_state, action.fx_index) {
                if !fx_state.enabled && auto_enabled_fx.insert((action.track, action.fx_index)) {
                    auto_enable_actions.push(PlannedAction::ToggleFx {
                        track: action.track,
                        fx_index: action.fx_index,
                        enabled: true,
                        reason: Some(format!("Enable '{}' before parameter edits", fx_state.name)),
                    });

                    diagnostics.push_preflight(format!(
                        "Auto-enabling '{}' on track {} before parameter edits.",
                        fx_state.name, action.track
                    ));
                }
            }
        }
    }

    // 6. ACTION FOCUS SUMMARY
    let mut category_counts: HashMap<&str, usize> = HashMap::new();
    for action in &deduplicated {
        if let Some(track_state) = find_track(&tracks_snapshot, action.track) {
            if let Some(fx_state) = find_fx(track_state, action.fx_index) {
                if let Some(param_state) = find_param(fx_state, action.param_index) {
                    let label =
                        category_label(ai_engine::SemanticAnalyzer::categorize(&param_state.name));
                    *category_counts.entry(label).or_default() += 1;
                }
            }
        }
    }

    if !category_counts.is_empty() {
        let mut focus_lines: Vec<String> = category_counts
            .into_iter()
            .map(|(label, count)| format!("{} x{}", label, count))
            .collect();
        focus_lines.sort();
        diagnostics.push_focus(format!("SetParam coverage: {}", focus_lines.join(", ")));
    }

    // Rebuild full action list: auto-enables → other_actions → optimized SetParam with enriched reasons
    plan.actions = auto_enable_actions;
    plan.actions.extend(other_actions);
    plan.actions.extend(
        deduplicated
            .into_iter()
            .map(|action| PlannedAction::SetParam {
                track: action.track,
                fx_index: action.fx_index,
                param_index: action.param_index,
                value: action.value,
                reason: Some(if action.reason.trim().is_empty() {
                    build_action_reason(&action, &tracks_snapshot)
                } else {
                    action.reason
                }),
            }),
    );

    println!(
        "[AI ENGINE] Final action count: {} (ready for execution)",
        plan.actions.len()
    );

    let engine_report = diagnostics.to_report();
    if let Some(report) = engine_report.as_ref() {
        plan.summary = format!("{}\n\n[AI Engine Report]\n{}", plan.summary, report);
    }

    // ========== UNDO SYSTEM: Begin recording changes ==========
    {
        let mut undo_manager = lock_or_recover(&state.undo_manager);
        undo_manager.begin_action(&format!("AI: {}", message.chars().take(50).collect::<String>()));
    }

    // Record changes for undo before applying
    for action in &plan.actions {
        match action {
            PlannedAction::SetParam {
                track,
                fx_index,
                param_index,
                value,
                ..
            } => {
                if let Some(track_state) = find_track(&tracks_snapshot, *track) {
                    if let Some(fx_state) = find_fx(track_state, *fx_index) {
                        if let Some(param_state) = find_param(fx_state, *param_index) {
                            let mut undo_manager = lock_or_recover(&state.undo_manager);
                            undo_manager.record_param_change(
                                *track,
                                *fx_index,
                                *param_index,
                                &param_state.name,
                                param_state.value,
                                *value,
                            );
                        }
                    }
                }
            }
            PlannedAction::ToggleFx {
                track,
                fx_index,
                enabled,
                ..
            } => {
                if let Some(track_state) = find_track(&tracks_snapshot, *track) {
                    if let Some(fx_state) = find_fx(track_state, *fx_index) {
                        let mut undo_manager = lock_or_recover(&state.undo_manager);
                        undo_manager.record_fx_toggle(
                            *track,
                            *fx_index,
                            &fx_state.name,
                            fx_state.enabled,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    let http_client = &state.http_client; // Clone is cheap for reqwest::Client
    let action_logs = apply_actions(&reaper, http_client, &tracks_snapshot, &plan.actions).await?;
    for log in &action_logs {
        println!("[AI ACTION] {}", log);
    }

    // ========== UNDO SYSTEM: Commit the action ==========
    {
        let mut undo_manager = lock_or_recover(&state.undo_manager);
        if let Some(action_id) = undo_manager.commit_action() {
            println!("[UNDO] Recorded action: {}", action_id);
        }
    }

    // ========== RECENT TONES: Add to history ==========
    add_recent_tone(
        &state,
        &message,
        &plan.summary,
        track_idx,
        plan.changes_table.len(),
    );

    {
        let mut history = lock_or_recover(&state.chat_history);
        push_history(
            &mut history,
            ChatMessage {
                role: "assistant".into(),
                content: plan.summary.clone(),
                timestamp: current_timestamp(),
            },
        );
    }

    let response = ChatResponse {
        summary: plan.summary,
        changes_table: plan.changes_table,
        engine_report,
        action_log: action_logs,
    };

    serde_json::to_string(&response).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_track_overview(state: State<'_, AppState>) -> Result<String, String> {
    let reaper = { lock_or_recover(&state.reaper).clone() };
    let tracks = reaper.get_tracks().await.map_err(|e| e.to_string())?;
    serde_json::to_string(&tracks).map_err(|e| e.to_string())
}

async fn capture_preset_with_params(
    name: &str,
    reaper: &ReaperClient,
) -> Result<PresetFile, String> {
    let tracks = reaper
        .get_tracks()
        .await
        .map_err(|e| format!("Failed to get tracks for preset: {e}"))?;

    let mut preset_tracks = Vec::new();
    for track in tracks.tracks {
        let mut fx_states = Vec::new();
        for fx in track.fx_list {
            let snapshot = reaper
                .get_fx_params(track.index, fx.index)
                .await
                .map_err(|e| {
                    format!(
                        "Failed to get params for track {} fx {}: {e}",
                        track.index, fx.index
                    )
                })?;

            let params = snapshot
                .params
                .into_iter()
                .map(|p| PresetParamState {
                    index: p.index,
                    name: p.name,
                    value: p.value,
                })
                .collect();

            fx_states.push(PresetFxState {
                index: fx.index,
                name: fx.name,
                params,
            });
        }

        preset_tracks.push(PresetTrackState {
            index: track.index,
            name: track.name,
            fx: fx_states,
        });
    }

    Ok(PresetFile {
        name: name.to_string(),
        created_at: current_timestamp(),
        project_path: None,
        tracks: preset_tracks,
    })
}

fn persist_preset_to_disk(preset: &PresetFile) -> Result<String, String> {
    let mut preset_dir = PathBuf::from("presets");
    fs::create_dir_all(&preset_dir)
        .map_err(|e| format!("Failed to create preset directory: {e}"))?;

    preset_dir.push(format!("{}.json", preset.name));
    let serialized = serde_json::to_string_pretty(preset)
        .map_err(|e| format!("Failed to serialize preset: {e}"))?;
    fs::write(&preset_dir, serialized).map_err(|e| format!("Failed to write preset file: {e}"))?;

    Ok(preset_dir.to_string_lossy().to_string())
}

async fn apply_preset_to_reaper(preset: &PresetFile, reaper: &ReaperClient) -> Result<(), String> {
    for track in &preset.tracks {
        for fx in &track.fx {
            for param in &fx.params {
                if let Err(err) = reaper
                    .set_param(track.index, fx.index, &param.name, param.value)
                    .await
                {
                    eprintln!(
                        "[PRESET] Failed to set param {} on track {} fx {}: {}",
                        param.name, track.index, fx.index, err
                    );
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
async fn set_fx_enabled(
    track: i32,
    fx: i32,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let reaper = { lock_or_recover(&state.reaper).clone() };
    reaper
        .set_fx_enabled(track, fx, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_preset(name: String, state: State<'_, AppState>) -> Result<String, String> {
    let reaper = { lock_or_recover(&state.reaper).clone() };
    let project_path = reaper
        .save_project(&name)
        .await
        .map_err(|e| e.to_string())?;

    let mut preset = capture_preset_with_params(&name, &reaper).await?;
    preset.project_path = Some(project_path.clone());
    let path = persist_preset_to_disk(&preset)?;

    println!(
        "[PRESET] Saved '{}' with {} tracks and {} FX to {} (project at {})",
        name,
        preset.tracks.len(),
        preset.tracks.iter().map(|t| t.fx.len()).sum::<usize>(),
        path,
        project_path
    );

    Ok(path)
}

#[tauri::command]
async fn load_preset(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let reaper = { lock_or_recover(&state.reaper).clone() };
    let preset_contents = fs::read_to_string(&path).map_err(|e| e.to_string());

    if let Ok(contents) = preset_contents {
        if let Ok(preset) = serde_json::from_str::<PresetFile>(&contents) {
            if let Some(project_path) = &preset.project_path {
                reaper
                    .load_project(project_path)
                    .await
                    .map_err(|e| format!("Failed to load project for preset: {e}"))?;
            }

            apply_preset_to_reaper(&preset, &reaper).await?;
            return Ok(format!(
                "Preset '{}' applied with {} tracks",
                preset.name,
                preset.tracks.len()
            ));
        }
    }

    reaper
        .load_project(&path)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Preset loaded from {}", path))
}

// ==================== UNDO/REDO COMMANDS ====================

#[tauri::command]
fn get_undo_state(state: State<'_, AppState>) -> Result<String, String> {
    let manager = lock_or_recover(&state.undo_manager);
    let undo_state = UndoState::from(&*manager);
    serde_json::to_string(&undo_state).map_err(|e| e.to_string())
}

#[tauri::command]
async fn perform_undo(state: State<'_, AppState>) -> Result<String, String> {
    let action = {
        let mut manager = lock_or_recover(&state.undo_manager);
        manager.pop_undo()
    };

    let Some(action) = action else {
        return Err("Nothing to undo".to_string());
    };

    let reaper = { lock_or_recover(&state.reaper).clone() };

    // Apply inverse of each change
    for change in &action.parameter_changes {
        if let Err(e) = reaper
            .set_param(change.track, change.fx_index, &change.param_name, change.old_value)
            .await
        {
            eprintln!("[UNDO] Failed to revert param: {}", e);
        }
    }

    for toggle in &action.fx_toggles {
        if let Err(e) = reaper
            .set_fx_enabled(toggle.track, toggle.fx_index, toggle.was_enabled)
            .await
        {
            eprintln!("[UNDO] Failed to revert toggle: {}", e);
        }
    }

    // For plugin changes, we'd need to add/remove plugins
    // This is more complex and may require REAPER extension support

    // Move action to redo stack
    {
        let mut manager = lock_or_recover(&state.undo_manager);
        manager.push_redo(action.clone());
    }

    Ok(format!("Undone: {}", action.description))
}

#[tauri::command]
async fn perform_redo(state: State<'_, AppState>) -> Result<String, String> {
    let action = {
        let mut manager = lock_or_recover(&state.undo_manager);
        manager.pop_redo()
    };

    let Some(action) = action else {
        return Err("Nothing to redo".to_string());
    };

    let reaper = { lock_or_recover(&state.reaper).clone() };

    // Re-apply each change
    for change in &action.parameter_changes {
        if let Err(e) = reaper
            .set_param(change.track, change.fx_index, &change.param_name, change.new_value)
            .await
        {
            eprintln!("[REDO] Failed to reapply param: {}", e);
        }
    }

    for toggle in &action.fx_toggles {
        if let Err(e) = reaper
            .set_fx_enabled(toggle.track, toggle.fx_index, !toggle.was_enabled)
            .await
        {
            eprintln!("[REDO] Failed to reapply toggle: {}", e);
        }
    }

    // Move action back to undo stack
    {
        let mut manager = lock_or_recover(&state.undo_manager);
        manager.push_undo(action.clone());
    }

    Ok(format!("Redone: {}", action.description))
}

#[tauri::command]
fn get_undo_history(state: State<'_, AppState>, limit: Option<usize>) -> Result<String, String> {
    let manager = lock_or_recover(&state.undo_manager);
    let history = manager.get_undo_history(limit.unwrap_or(10));
    serde_json::to_string(&history).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_undo_history(state: State<'_, AppState>) -> Result<(), String> {
    let mut manager = lock_or_recover(&state.undo_manager);
    manager.clear();
    Ok(())
}

// ==================== RECENT TONES COMMANDS ====================

#[tauri::command]
fn get_recent_tones(state: State<'_, AppState>) -> Result<String, String> {
    let recent = lock_or_recover(&state.recent_tones);
    serde_json::to_string(&*recent).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_recent_tones(state: State<'_, AppState>) -> Result<(), String> {
    let mut recent = lock_or_recover(&state.recent_tones);
    recent.clear();
    Ok(())
}

fn add_recent_tone(state: &AppState, query: &str, summary: &str, track: i32, changes_count: usize) {
    let mut recent = lock_or_recover(&state.recent_tones);

    let tone = RecentTone {
        id: uuid::Uuid::new_v4().to_string(),
        query: query.to_string(),
        summary: summary.to_string(),
        timestamp: current_timestamp(),
        track,
        changes_count,
    };

    // Add to front
    recent.insert(0, tone);

    // Keep only MAX_RECENT_TONES
    recent.truncate(MAX_RECENT_TONES);
}

// ==================== EXPORT COMMANDS ====================

#[tauri::command]
fn export_tone_as_text(
    changes: Vec<ChangeEntry>,
    summary: String,
) -> Result<String, String> {
    let mut output = String::new();

    output.push_str("═══════════════════════════════════════\n");
    output.push_str("        TONEFORGE - TONE EXPORT\n");
    output.push_str("═══════════════════════════════════════\n\n");

    output.push_str(&format!("Summary: {}\n\n", summary));

    output.push_str("Changes Applied:\n");
    output.push_str("───────────────────────────────────────\n");

    for (i, change) in changes.iter().enumerate() {
        output.push_str(&format!(
            "{}. {} - {}\n   {} → {}\n   Reason: {}\n\n",
            i + 1,
            change.plugin,
            change.parameter,
            change.old_value,
            change.new_value,
            change.reason
        ));
    }

    output.push_str("───────────────────────────────────────\n");
    output.push_str(&format!("Total Changes: {}\n", changes.len()));
    output.push_str(&format!("Exported: {}\n", chrono_lite_now()));
    output.push_str("Generated by ToneForge\n");

    Ok(output)
}

fn chrono_lite_now() -> String {
    let secs = current_timestamp();
    // Simple timestamp formatting without chrono dependency
    format!("Unix timestamp: {}", secs)
}

// ==================== SECURE STORAGE COMMANDS ====================

#[tauri::command]
fn save_api_config(
    api_key: String,
    provider: String,
    model: String,
    custom_instructions: Option<String>,
) -> Result<(), String> {
    let config = secure_storage::SecureConfig {
        api_key: Some(api_key),
        provider: Some(provider),
        model: Some(model),
        custom_instructions,
    };

    secure_storage::save_config(&config)
}

#[tauri::command]
fn load_api_config() -> Result<String, String> {
    let config = secure_storage::load_config()?;
    serde_json::to_string(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_api_config() -> Result<(), String> {
    secure_storage::delete_config()
}

#[tauri::command]
fn has_saved_api_config() -> bool {
    secure_storage::config_exists()
}

#[tauri::command]
fn mask_api_key(key: String) -> String {
    secure_storage::mask_api_key(&key)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            reaper: Mutex::new(ReaperClient::new()),
            ai_provider: Mutex::new(None),
            chat_history: Mutex::new(Vec::new()),
            http_client: reqwest::Client::new(), // Shared HTTP client
            tone_researcher: ToneResearcher::new(), // First AI layer for tone research
            undo_manager: Mutex::new(UndoManager::new()), // Undo/Redo system
            recent_tones: Mutex::new(Vec::new()), // Recent tones history
        })
        .invoke_handler(tauri::generate_handler![
            check_reaper_connection,
            configure_ai_provider,
            get_chat_history,
            process_chat_message,
            get_track_overview,
            set_fx_enabled,
            save_preset,
            load_preset,
            load_reference_audio,
            load_input_audio,
            calculate_eq_match,
            export_eq_settings,
            // Undo/Redo commands
            get_undo_state,
            perform_undo,
            perform_redo,
            get_undo_history,
            clear_undo_history,
            // Recent tones commands
            get_recent_tones,
            clear_recent_tones,
            // Export commands
            export_tone_as_text,
            // Secure storage commands
            save_api_config,
            load_api_config,
            delete_api_config,
            has_saved_api_config,
            mask_api_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
