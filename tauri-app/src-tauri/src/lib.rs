//! ToneForge v2 - Two-Tier AI Tone Generation System
//!
//! Architecture:
//! - Tier 1 (Tone AI): Searches encyclopedia or generates tone recommendations
//! - Tier 2 (Parameter AI): Maps tone parameters to REAPER with precision

mod ai_client;
mod audio;
mod dsp;
mod errors;
mod parameter_ai;
mod reaper_client;
mod secure_storage;
mod tone_ai;
mod tone_encyclopedia;
mod undo_redo;

use ai_client::AIProvider;
use audio::analyzer::{analyze_spectrum, AnalysisConfig};
use audio::loader::{load_audio_file, resample_audio};
use audio::matcher::{match_profiles, MatchConfig as EqMatchConfig, MatchResult as EqMatchResult};
use audio::profile::{extract_eq_profile, EQProfile};
use parameter_ai::{ParameterAction, ParameterAI, ReaperParameter, ReaperPlugin, ReaperSnapshot};
use reaper_client::ReaperClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tone_ai::{ToneAI, ToneAIResult, ToneSource};
use tone_encyclopedia::ToneEncyclopedia;
use undo_redo::{UndoManager, UndoState};

const ENCYCLOPEDIA_PATH: &str = "tone_encyclopedia.json";

// ==================== APP STATE ====================

struct AppState {
    reaper: Mutex<ReaperClient>,
    ai_provider: Mutex<Option<AIProvider>>,
    tone_encyclopedia: Mutex<ToneEncyclopedia>,
    undo_manager: Mutex<UndoManager>,
}

// ==================== TAURI COMMANDS ====================

#[tauri::command]
async fn check_reaper_connection(state: State<'_, AppState>) -> Result<bool, String> {
    let reaper = state.reaper.lock().unwrap();
    reaper.ping().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn configure_ai_provider(
    provider_name: String,
    model: String,
    api_key: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let provider = match provider_name.to_lowercase().as_str() {
        "openai" | "gpt" => AIProvider::openai(api_key, model.clone()),
        "claude" | "anthropic" => AIProvider::claude(api_key, model.clone()),
        "gemini" | "google" => AIProvider::gemini(api_key, model.clone()),
        "grok" | "xai" => AIProvider::grok(api_key, model.clone()),
        _ => return Err(format!("Unsupported provider: {}", provider_name)),
    };

    let mut guard = state.ai_provider.lock().unwrap();
    *guard = Some(provider.clone());

    Ok(format!(
        "{} configured with model {}",
        provider.name(),
        provider.model_name()
    ))
}

#[tauri::command]
async fn process_tone_request(
    message: String,
    track: Option<i32>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    println!("\n========== TWO-TIER AI PIPELINE START ==========");
    println!("[USER] {}", message);

    let track_idx = track.unwrap_or(0);

    // Get AI provider
    let ai_provider = {
        let guard = state.ai_provider.lock().unwrap();
        guard
            .clone()
            .ok_or_else(|| "AI provider not configured".to_string())?
    };

    // ========== TIER 1: TONE AI ==========
    println!("\n[TIER 1] Running Tone AI...");

    let tone_ai = {
        let encyclopedia = state.tone_encyclopedia.lock().unwrap().clone();
        ToneAI::new(encyclopedia).with_ai_provider(ai_provider.clone())
    };

    let tone_result = tone_ai
        .process_request(&message)
        .await
        .map_err(|e| format!("Tone AI error: {}", e))?;

    println!("[TIER 1] Result:");
    println!("  - Source: {:?}", tone_result.source);
    println!("  - Description: {}", tone_result.tone_description);
    println!("  - Confidence: {:.0}%", tone_result.confidence * 100.0);

    // ========== GET REAPER SNAPSHOT ==========
    println!("\n[REAPER] Fetching current state...");

    let reaper_snapshot = {
        let reaper = state.reaper.lock().unwrap();
        collect_reaper_snapshot(&reaper, track_idx)
            .await
            .map_err(|e| format!("Failed to get REAPER state: {}", e))?
    };

    println!("[REAPER] Track: {}", reaper_snapshot.track_name);
    println!("[REAPER] Plugins: {}", reaper_snapshot.plugins.len());

    // ========== TIER 2: PARAMETER AI ==========
    println!("\n[TIER 2] Running Parameter AI...");

    let parameter_ai = ParameterAI::new(ai_provider);

    let parameter_result = parameter_ai
        .map_parameters(
            &tone_result.parameters,
            &reaper_snapshot,
            &tone_result.tone_description,
        )
        .await
        .map_err(|e| format!("Parameter AI error: {}", e))?;

    println!("[TIER 2] Generated {} actions", parameter_result.actions.len());
    println!("[TIER 2] Summary: {}", parameter_result.summary);

    if !parameter_result.warnings.is_empty() {
        println!("[TIER 2] Warnings:");
        for warning in &parameter_result.warnings {
            println!("  ⚠️  {}", warning);
        }
    }

    // ========== VALIDATE ACTIONS ==========
    let validation_warnings = parameter_ai.validate_actions(&parameter_result.actions, &reaper_snapshot);

    if !validation_warnings.is_empty() {
        println!("\n[VALIDATION] Warnings:");
        for warning in &validation_warnings {
            println!("  ⚠️  {}", warning);
        }
    }

    // ========== RECORD FOR UNDO ==========
    {
        let mut undo_manager = state.undo_manager.lock().unwrap();
        undo_manager.begin_action(&format!("Tone: {}", message));
    }

    // ========== APPLY ACTIONS TO REAPER ==========
    println!("\n[APPLY] Applying actions to REAPER...");

    let action_logs = {
        let reaper = state.reaper.lock().unwrap();
        apply_parameter_actions(&reaper, &parameter_result.actions, &reaper_snapshot, &mut state.undo_manager.lock().unwrap())
            .await
            .map_err(|e| format!("Failed to apply actions: {}", e))?
    };

    for log in &action_logs {
        println!("[ACTION] {}", log);
    }

    // ========== COMMIT UNDO ==========
    {
        let mut undo_manager = state.undo_manager.lock().unwrap();
        if let Some(action_id) = undo_manager.commit_action() {
            println!("[UNDO] Recorded action: {}", action_id);
        }
    }

    println!("\n========== TWO-TIER AI PIPELINE COMPLETE ==========\n");

    // Build response
    let response = ToneResponse {
        tone_source: format!("{:?}", tone_result.source),
        tone_description: tone_result.tone_description,
        confidence: tone_result.confidence,
        summary: parameter_result.summary,
        actions_count: parameter_result.actions.len(),
        action_logs,
        warnings: parameter_result.warnings,
        validation_warnings,
    };

    serde_json::to_string(&response).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct ToneResponse {
    tone_source: String,
    tone_description: String,
    confidence: f32,
    summary: String,
    actions_count: usize,
    action_logs: Vec<String>,
    warnings: Vec<String>,
    validation_warnings: Vec<String>,
}

// ==================== REAPER SNAPSHOT COLLECTION ====================

async fn collect_reaper_snapshot(
    reaper: &ReaperClient,
    track_idx: i32,
) -> Result<ReaperSnapshot, Box<dyn Error>> {
    let overview = reaper.get_tracks().await?;

    let track = overview
        .tracks
        .iter()
        .find(|t| t.index == track_idx)
        .ok_or_else(|| format!("Track {} not found", track_idx))?;

    let mut plugins = Vec::new();

    for fx in &track.fx_list {
        let params_snapshot = reaper.get_fx_params(track_idx, fx.index).await?;

        let parameters: Vec<ReaperParameter> = params_snapshot
            .params
            .into_iter()
            .map(|p| ReaperParameter {
                index: p.index,
                name: p.name,
                current_value: p.value,
                display_value: p.display,
            })
            .collect();

        plugins.push(ReaperPlugin {
            index: fx.index,
            name: fx.name.clone(),
            enabled: fx.enabled,
            parameters,
        });
    }

    Ok(ReaperSnapshot {
        track_index: track_idx,
        track_name: track.name.clone(),
        plugins,
    })
}

// ==================== APPLY PARAMETER ACTIONS ====================

async fn apply_parameter_actions(
    reaper: &ReaperClient,
    actions: &[ParameterAction],
    snapshot: &ReaperSnapshot,
    undo_manager: &mut UndoManager,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut logs = Vec::new();

    for action in actions {
        match action {
            ParameterAction::SetParameter {
                track,
                plugin_index,
                param_index,
                param_name,
                value,
                reason,
            } => {
                // Find current value for undo
                if let Some(plugin) = snapshot.plugins.iter().find(|p| p.index == *plugin_index) {
                    if let Some(param) = plugin.parameters.iter().find(|p| p.index == *param_index) {
                        // Record for undo
                        undo_manager.record_param_change(
                            *track,
                            *plugin_index,
                            *param_index,
                            param_name,
                            param.current_value,
                            *value,
                        );

                        // Apply change
                        reaper.set_param(*track, *plugin_index, param_name, *value).await?;

                        logs.push(format!(
                            "✓ {} :: {} = {:.1}% (was {:.1}%) - {}",
                            plugin.name,
                            param_name,
                            value * 100.0,
                            param.current_value * 100.0,
                            reason
                        ));
                    }
                }
            }
            ParameterAction::EnablePlugin {
                track,
                plugin_index,
                plugin_name,
                reason,
            } => {
                // Find current state for undo
                if let Some(plugin) = snapshot.plugins.iter().find(|p| p.index == *plugin_index) {
                    undo_manager.record_fx_toggle(*track, *plugin_index, plugin_name, plugin.enabled);
                }

                reaper.set_fx_enabled(*track, *plugin_index, true).await?;

                logs.push(format!("✓ Enabled '{}' - {}", plugin_name, reason));
            }
            ParameterAction::LoadPlugin {
                track,
                plugin_name,
                reason,
                ..
            } => {
                let slot = reaper.add_plugin(*track, plugin_name).await?;

                logs.push(format!(
                    "✓ Loaded '{}' at slot {} - {}",
                    plugin_name, slot, reason
                ));
            }
        }
    }

    Ok(logs)
}

// ==================== ENCYCLOPEDIA MANAGEMENT ====================

#[tauri::command]
async fn load_encyclopedia(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let encyclopedia = ToneEncyclopedia::load_from_file(&path)?;

    let count = encyclopedia.count();

    let mut guard = state.tone_encyclopedia.lock().unwrap();
    *guard = encyclopedia;

    Ok(format!("Loaded {} tones from encyclopedia", count))
}

#[tauri::command]
async fn get_encyclopedia_stats(state: State<'_, AppState>) -> Result<String, String> {
    let encyclopedia = state.tone_encyclopedia.lock().unwrap();

    let stats = serde_json::json!({
        "total_tones": encyclopedia.count(),
        "genres": encyclopedia.get_all_genres(),
        "artists": encyclopedia.get_all_artists(),
    });

    serde_json::to_string(&stats).map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_encyclopedia(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let encyclopedia = state.tone_encyclopedia.lock().unwrap();

    let results = encyclopedia.search(&query, limit.unwrap_or(10));

    #[derive(Serialize)]
    struct SearchResultResponse {
        id: String,
        artist: String,
        album: Option<String>,
        song: Option<String>,
        description: String,
        score: f32,
        matched_fields: Vec<String>,
    }

    let response: Vec<SearchResultResponse> = results
        .into_iter()
        .map(|r| SearchResultResponse {
            id: r.tone.id.clone(),
            artist: r.tone.artist.clone(),
            album: r.tone.album.clone(),
            song: r.tone.song.clone(),
            description: r.tone.description.clone(),
            score: r.score,
            matched_fields: r.matched_fields,
        })
        .collect();

    serde_json::to_string(&response).map_err(|e| e.to_string())
}

// ==================== UNDO/REDO COMMANDS ====================

#[tauri::command]
fn get_undo_state(state: State<'_, AppState>) -> Result<String, String> {
    let manager = state.undo_manager.lock().unwrap();
    let undo_state = UndoState::from(&*manager);
    serde_json::to_string(&undo_state).map_err(|e| e.to_string())
}

#[tauri::command]
async fn perform_undo(state: State<'_, AppState>) -> Result<String, String> {
    let action = {
        let mut manager = state.undo_manager.lock().unwrap();
        manager.pop_undo()
    };

    let Some(action) = action else {
        return Err("Nothing to undo".to_string());
    };

    let reaper = state.reaper.lock().unwrap();

    // Apply inverse of each change
    for change in &action.parameter_changes {
        if let Err(e) = reaper
            .set_param(
                change.track,
                change.fx_index,
                &change.param_name,
                change.old_value,
            )
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

    // Move action to redo stack
    {
        let mut manager = state.undo_manager.lock().unwrap();
        manager.push_redo(action.clone());
    }

    Ok(format!("Undone: {}", action.description))
}

#[tauri::command]
async fn perform_redo(state: State<'_, AppState>) -> Result<String, String> {
    let action = {
        let mut manager = state.undo_manager.lock().unwrap();
        manager.pop_redo()
    };

    let Some(action) = action else {
        return Err("Nothing to redo".to_string());
    };

    let reaper = state.reaper.lock().unwrap();

    // Re-apply each change
    for change in &action.parameter_changes {
        if let Err(e) = reaper
            .set_param(
                change.track,
                change.fx_index,
                &change.param_name,
                change.new_value,
            )
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
        let mut manager = state.undo_manager.lock().unwrap();
        manager.push_undo(action.clone());
    }

    Ok(format!("Redone: {}", action.description))
}

// ==================== AUDIO ANALYSIS (EQ MATCH) ====================

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

// ==================== SECURE STORAGE ====================

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

// ==================== MAIN APP ====================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Try to load encyclopedia on startup
    let encyclopedia = ToneEncyclopedia::load_from_file(ENCYCLOPEDIA_PATH)
        .unwrap_or_else(|e| {
            println!("[STARTUP] Failed to load encyclopedia: {}", e);
            println!("[STARTUP] Using empty encyclopedia");
            ToneEncyclopedia::new()
        });

    println!("[STARTUP] Encyclopedia loaded: {} tones", encyclopedia.count());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            reaper: Mutex::new(ReaperClient::new()),
            ai_provider: Mutex::new(None),
            tone_encyclopedia: Mutex::new(encyclopedia),
            undo_manager: Mutex::new(UndoManager::new()),
        })
        .invoke_handler(tauri::generate_handler![
            // Connection
            check_reaper_connection,
            // AI Configuration
            configure_ai_provider,
            // Tone Processing (Main Feature)
            process_tone_request,
            // Encyclopedia Management
            load_encyclopedia,
            get_encyclopedia_stats,
            search_encyclopedia,
            // Undo/Redo
            get_undo_state,
            perform_undo,
            perform_redo,
            // Audio Analysis
            load_reference_audio,
            load_input_audio,
            calculate_eq_match,
            // Secure Storage
            save_api_config,
            load_api_config,
            delete_api_config,
            has_saved_api_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
