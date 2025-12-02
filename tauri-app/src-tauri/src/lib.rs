mod audio;
mod dsp;
mod gemini_client;
mod reaper_client;

use audio::analyzer::{analyze_spectrum, AnalysisConfig};
use audio::loader::{load_audio_file, resample_audio};
use audio::matcher::{match_profiles, MatchConfig as EqMatchConfig, MatchResult as EqMatchResult};
use audio::profile::{extract_eq_profile, EQProfile};
use gemini_client::GeminiClient;
use reaper_client::ReaperClient;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

const MAX_HISTORY: usize = 40;
const PROMPT_HISTORY_LIMIT: usize = 12;
const SYSTEM_PROMPT: &str = r#"
You are ToneForge's autonomous tone engineer. You always see the complete FX chain and every parameter snapshot.
Rules:
1. Always respond with pure JSON only. Shape:
{
  "summary": "<brief overall description of what was done in user's language>",
  "changes_table": [
    { "plugin": "<plugin name>", "parameter": "<param name>", "old_value": "<previous>", "new_value": "<current>", "reason": "<why>" }
  ],
  "actions": [
    { "type": "set_param", "track": <track_index>, "fx_index": <fx_index>, "param_index": <param_index>, "value": <0.0-1.0>, "reason": "..." },
    { "type": "toggle_fx", "track": <track_index>, "fx_index": <fx_index>, "enabled": true, "reason": "..." },
    { "type": "load_plugin", "track": <track_index>, "plugin_name": "ReaEQ", "position": 1, "reason": "..." },
    { "type": "noop", "reason": "why no changes" }
  ]
}
2. Keep summary concise (1-2 sentences max). The changes_table shows the details.
3. In changes_table, list every modification with plugin name, parameter name, old/new values, and brief reason.
4. Indices must come from the provided track/fx data. Do not invent parameters.
5. If nothing should change, return empty actions and changes_table but explain why in summary.
6. Keep tone concise and technical.
7. The provided JSON snapshot already includes everything you need. Do not fabricate new state.
8. CRITICAL - ALWAYS verify that controls are ENABLED/ACTIVE before modifying their parameters:
   - Before changing any parameter, check if its parent module/section/channel is currently active
   - Look for enable/bypass/on-off parameters (often param_index 0 or named "Enable"/"Bypass"/"Active")
   - If the target is disabled: either enable it first OR modify the currently active alternative instead
   - Example: Don't adjust "Hot EQ" parameters if "Clean EQ" is the active mode
   - Example: Don't modify "Channel B" settings if "Channel A" is selected
   - NEVER assume a control is active just because it exists - verify its state from the parameter snapshot
   - If uncertain about enable state, add a toggle_fx or set_param action to enable it before adjustment
9. PLUGIN SELECTION INTELLIGENCE - Choose industry-standard, professional-grade plugins:
   - Research and prefer modern, widely-used professional plugins over outdated alternatives
   - Prioritize quality
   - When multiple options exist for the same task, choose the one with better reputation in professional production
   - Consider the user's available plugins but recommend upgrades when current options are subpar
   - Don't load inferior plugins when superior alternatives are available in the chain
   - Stay current with industry standards - prefer tools actually used in modern professional studios
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
}

#[derive(Clone, Serialize)]
struct FxParamState {
    index: i32,
    name: String,
    value: f64,
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

#[derive(Serialize)]
struct PromptPayload {
    selected_track: i32,
    tracks: Vec<TrackSnapshot>,
    recent_messages: Vec<ConversationEntry>,
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
}

#[derive(Debug, Serialize, Deserialize)]
struct AIPlan {
    summary: String,
    #[serde(default)]
    changes_table: Vec<ChangeEntry>,
    #[serde(default)]
    actions: Vec<PlannedAction>,
}

#[derive(Debug, Serialize, Deserialize)]
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
    Ok(format!("{SYSTEM_PROMPT}\n\n=== SNAPSHOT START ===\n{}\n=== SNAPSHOT END ===", context))
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

fn find_track<'a>(tracks: &'a [TrackSnapshot], track_idx: i32) -> Option<&'a TrackSnapshot> {
    tracks.iter().find(|t| t.index == track_idx)
}

fn find_fx<'a>(track: &'a TrackSnapshot, fx_idx: i32) -> Option<&'a FxState> {
    track.fx.iter().find(|fx| fx.index == fx_idx)
}

fn find_param<'a>(fx: &'a FxState, param_idx: i32) -> Option<&'a FxParamState> {
    fx.params.iter().find(|p| p.index == param_idx)
}

async fn apply_actions(
    reaper: &ReaperClient,
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
                        if let Some(param_state) = find_param(fx_state, *param_index) {
                            reaper
                                .set_param(*track, *fx_index, &param_state.name, *value)
                                .await
                                .map_err(|e| e.to_string())?;
                            logs.push(format!(
                                "{} :: {} -> {} = {:.1}% ({})",
                                track_state.name,
                                fx_state.name,
                                param_state.name,
                                value * 100.0,
                                reason.clone().unwrap_or_else(|| "no reason".into())
                            ));
                        } else {
                            logs.push(format!(
                                "Skipped set_param: param {} not found on {}",
                                param_index, fx_state.name
                            ));
                        }
                    } else {
                        logs.push(format!(
                            "Skipped set_param: fx {} not found on track {}",
                            fx_index, track_state.name
                        ));
                    }
                } else {
                    logs.push(format!("Skipped set_param: track {} missing", track));
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
                    "Track {} FX {} toggled to {} ({})",
                    track,
                    fx_index,
                    enabled,
                    reason.clone().unwrap_or_else(|| "no reason".into())
                ));
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
                    "Loaded '{}' on track {} slot {} ({})",
                    plugin_name,
                    track,
                    slot,
                    reason.clone().unwrap_or_else(|| "no reason".into())
                ));
            }
            PlannedAction::Noop { reason } => {
                logs.push(format!(
                    "AI noop: {}",
                    reason.clone().unwrap_or_else(|| "no changes".into())
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
        "json" => serde_json::to_string_pretty(&result.correction_profile).map_err(|e| e.to_string()),
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

        let freq_norm = (band.frequency.log2() - 20.0f32.log2())
            / (20_000.0f32.log2() - 20.0f32.log2());
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
    let reaper = { state.reaper.lock().unwrap().clone() };
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
        let mut guard = state.ai_provider.lock().unwrap();
        *guard = Some(ai_provider);
    }

    {
        let mut history = state.chat_history.lock().unwrap();
        history.clear();
    }

    Ok(format!("{} configured (model: {})", provider, model))
}

#[tauri::command]
fn get_chat_history(state: State<'_, AppState>) -> Result<String, String> {
    let history = state.chat_history.lock().unwrap();
    serde_json::to_string(&*history).map_err(|e| e.to_string())
}

#[tauri::command]
async fn process_chat_message(
    message: String,
    track: Option<i32>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let ai_provider = {
        let guard = state.ai_provider.lock().unwrap();
        guard
            .clone()
            .ok_or_else(|| "AI provider is not configured".to_string())?
    };

    let track_idx = track.unwrap_or(0).max(0);

    {
        let mut history = state.chat_history.lock().unwrap();
        push_history(
            &mut history,
            ChatMessage {
                role: "user".into(),
                content: message.clone(),
                timestamp: current_timestamp(),
            },
        );
    }

    let reaper = { state.reaper.lock().unwrap().clone() };
    let tracks_snapshot = collect_track_snapshots(&reaper).await?;

    let history_snapshot = {
        let history = state.chat_history.lock().unwrap();
        conversation_for_prompt(&history)
    };

    let payload = PromptPayload {
        selected_track: track_idx,
        tracks: tracks_snapshot.clone(),
        recent_messages: history_snapshot,
    };

    let prompt = build_prompt(&payload)?;
    let ai_text = ai_provider.generate(&prompt).await.map_err(|e| e.to_string())?;
    let plan = parse_plan(&ai_text)?;

    let action_logs = apply_actions(&reaper, &tracks_snapshot, &plan.actions).await?;
    for log in action_logs {
        println!("[AI ACTION] {}", log);
    }

    {
        let mut history = state.chat_history.lock().unwrap();
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
    };

    serde_json::to_string(&response).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_track_overview(state: State<'_, AppState>) -> Result<String, String> {
    let reaper = { state.reaper.lock().unwrap().clone() };
    let tracks = reaper.get_tracks().await.map_err(|e| e.to_string())?;
    serde_json::to_string(&tracks).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_fx_enabled(
    track: i32,
    fx: i32,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let reaper = { state.reaper.lock().unwrap().clone() };
    reaper
        .set_fx_enabled(track, fx, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_preset(name: String, state: State<'_, AppState>) -> Result<String, String> {
    let reaper = { state.reaper.lock().unwrap().clone() };
    let path = reaper
        .save_project(&name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
async fn load_preset(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let reaper = { state.reaper.lock().unwrap().clone() };
    reaper
        .load_project(&path)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("Preset loaded from {}", path))
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
