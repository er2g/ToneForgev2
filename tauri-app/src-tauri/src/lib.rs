//! ToneForge v2 - Multi-Mode AI Conversation System
//!
//! Three AI Modes:
//! - 🔍 Researcher: Tone research and discussion (no REAPER)
//! - 📋 Planner: Analysis and suggestions (read-only REAPER)
//! - ⚡ Act: Direct application (full two-tier system)
//!
//! Each mode operates in independent conversation rooms!

mod act_mode;
mod ai_client;
mod audio;
mod conversation;
mod dsp;
mod errors;
mod parameter_ai;
mod planner_mode;
mod reaper_client;
mod researcher_mode;
mod secure_storage;
mod tone_ai;
mod tone_encyclopedia;
mod undo_redo;

use act_mode::ActMode;
use ai_client::AIProvider;
use audio::analyzer::{analyze_spectrum, AnalysisConfig};
use audio::loader::{load_audio_file, resample_audio};
use audio::matcher::{match_profiles, MatchConfig as EqMatchConfig, MatchResult as EqMatchResult};
use audio::profile::{extract_eq_profile, EQProfile};
use conversation::{Conversation, ConversationManager, ConversationMode, ConversationSummary, Message, MessageMetadata, MessageRole};
use planner_mode::PlannerMode;
use reaper_client::ReaperClient;
use researcher_mode::ResearcherMode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;
use tauri::State;
use tone_encyclopedia::ToneEncyclopedia;
use undo_redo::{UndoManager, UndoState};

const ENCYCLOPEDIA_PATH: &str = "tone_encyclopedia.json";

// ==================== APP STATE ====================

struct AppState {
    reaper: Mutex<ReaperClient>,
    ai_provider: Mutex<Option<AIProvider>>,
    tone_encyclopedia: Mutex<ToneEncyclopedia>,
    undo_manager: Mutex<UndoManager>,
    conversation_manager: Mutex<ConversationManager>,
}

// ==================== AI CONFIGURATION ====================

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

// ==================== CONVERSATION MANAGEMENT ====================

#[tauri::command]
fn create_conversation(
    title: String,
    mode: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let conversation_mode = match mode.to_lowercase().as_str() {
        "researcher" => ConversationMode::Researcher,
        "planner" => ConversationMode::Planner,
        "act" => ConversationMode::Act,
        _ => return Err(format!("Unknown mode: {}", mode)),
    };

    let mut manager = state.conversation_manager.lock().unwrap();
    let id = manager.create_conversation(title, conversation_mode);

    Ok(id)
}

#[tauri::command]
fn list_conversations(state: State<'_, AppState>) -> Result<String, String> {
    let manager = state.conversation_manager.lock().unwrap();
    let conversations = manager.list_active_conversations();

    let summaries: Vec<ConversationSummary> = conversations
        .iter()
        .map(|c| ConversationSummary::from(*c))
        .collect();

    serde_json::to_string(&summaries).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = state.conversation_manager.lock().unwrap();

    let conversation = manager
        .get_conversation(&conversation_id)
        .ok_or_else(|| "Conversation not found".to_string())?;

    serde_json::to_string(conversation).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut manager = state.conversation_manager.lock().unwrap();

    if manager.delete_conversation(&conversation_id) {
        Ok(())
    } else {
        Err("Conversation not found".to_string())
    }
}

#[tauri::command]
fn clear_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut manager = state.conversation_manager.lock().unwrap();
    manager.clear_conversation(&conversation_id)
}

// ==================== MESSAGE PROCESSING ====================

#[tauri::command]
async fn send_message(
    conversation_id: String,
    message: String,
    track_index: Option<i32>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get conversation details
    let (mode, conversation_history) = {
        let manager = state.conversation_manager.lock().unwrap();
        let conversation = manager
            .get_conversation(&conversation_id)
            .ok_or_else(|| "Conversation not found".to_string())?;

        let history: Vec<Message> = conversation.messages.clone();
        (conversation.mode, history)
    };

    // Add user message to conversation
    {
        let mut manager = state.conversation_manager.lock().unwrap();
        manager.add_message(&conversation_id, MessageRole::User, message.clone(), None)?;
    }

    // Get AI provider
    let ai_provider = {
        let guard = state.ai_provider.lock().unwrap();
        guard
            .clone()
            .ok_or_else(|| "AI provider not configured".to_string())?
    };

    // Process based on mode
    let response_data = match mode {
        ConversationMode::Researcher => {
            process_researcher_message(&message, &conversation_history, &state, ai_provider).await?
        }
        ConversationMode::Planner => {
            let track = track_index.unwrap_or(0);
            process_planner_message(&message, &conversation_history, track, &state, ai_provider).await?
        }
        ConversationMode::Act => {
            let track = track_index.unwrap_or(0);
            process_act_message(&message, track, &state, ai_provider).await?
        }
    };

    // Add assistant response to conversation
    {
        let mut manager = state.conversation_manager.lock().unwrap();
        manager.add_message(
            &conversation_id,
            MessageRole::Assistant,
            response_data.content.clone(),
            response_data.metadata,
        )?;
    }

    Ok(serde_json::to_string(&response_data).map_err(|e| e.to_string())?)
}

#[derive(Serialize)]
struct MessageResponseData {
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<MessageMetadata>,
}

async fn process_researcher_message(
    message: &str,
    history: &[Message],
    state: &State<'_, AppState>,
    ai_provider: AIProvider,
) -> Result<MessageResponseData, String> {
    let encyclopedia = state.tone_encyclopedia.lock().unwrap().clone();

    let researcher = ResearcherMode::new(encyclopedia, ai_provider);

    let history_refs: Vec<&Message> = history.iter().collect();
    let response = researcher.process_message(message, &history_refs).await?;

    let metadata = MessageMetadata {
        actions_count: None,
        encyclopedia_matches: Some(response.encyclopedia_matches.len()),
        suggestions_count: Some(response.suggestions.len()),
        notes: if response.suggestions.is_empty() {
            None
        } else {
            Some(response.suggestions)
        },
    };

    Ok(MessageResponseData {
        content: response.content,
        metadata: Some(metadata),
    })
}

async fn process_planner_message(
    message: &str,
    history: &[Message],
    track_index: i32,
    state: &State<'_, AppState>,
    ai_provider: AIProvider,
) -> Result<MessageResponseData, String> {
    let reaper = state.reaper.lock().unwrap().clone();

    let planner = PlannerMode::new(reaper, ai_provider);

    let history_refs: Vec<&Message> = history.iter().collect();
    let response = planner.process_message(message, &history_refs, track_index).await?;

    let metadata = MessageMetadata {
        actions_count: None,
        encyclopedia_matches: None,
        suggestions_count: Some(response.suggestions.len()),
        notes: Some(vec![response.current_state_summary]),
    };

    Ok(MessageResponseData {
        content: response.content,
        metadata: Some(metadata),
    })
}

async fn process_act_message(
    message: &str,
    track_index: i32,
    state: &State<'_, AppState>,
    ai_provider: AIProvider,
) -> Result<MessageResponseData, String> {
    let encyclopedia = state.tone_encyclopedia.lock().unwrap().clone();
    let reaper = state.reaper.lock().unwrap().clone();

    let act_mode = ActMode::new(encyclopedia, reaper, ai_provider);

    let mut undo_manager = state.undo_manager.lock().unwrap();
    let response = act_mode.process_message(message, track_index, &mut undo_manager).await?;

    let content = format!(
        "**{}**\n\n{}\n\n**Actions Applied**: {}\n\n**Details**:\n{}",
        response.tone_description,
        response.summary,
        response.actions_count,
        response.action_logs.join("\n")
    );

    let mut notes = response.action_logs;
    notes.extend(response.warnings);

    let metadata = MessageMetadata {
        actions_count: Some(response.actions_count),
        encyclopedia_matches: None,
        suggestions_count: None,
        notes: Some(notes),
    };

    Ok(MessageResponseData {
        content,
        metadata: Some(metadata),
    })
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

// ==================== UNDO/REDO ====================

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
    // Load encyclopedia on startup
    let encyclopedia = ToneEncyclopedia::load_from_file(ENCYCLOPEDIA_PATH)
        .unwrap_or_else(|e| {
            println!("[STARTUP] Failed to load encyclopedia: {}", e);
            println!("[STARTUP] Using empty encyclopedia");
            ToneEncyclopedia::new()
        });

    println!("[STARTUP] Encyclopedia loaded: {} tones", encyclopedia.count());
    println!("[STARTUP] Multi-mode conversation system initialized");
    println!("[STARTUP] Modes: 🔍 Researcher | 📋 Planner | ⚡ Act");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            reaper: Mutex::new(ReaperClient::new()),
            ai_provider: Mutex::new(None),
            tone_encyclopedia: Mutex::new(encyclopedia),
            undo_manager: Mutex::new(UndoManager::new()),
            conversation_manager: Mutex::new(ConversationManager::new()),
        })
        .invoke_handler(tauri::generate_handler![
            // Connection
            check_reaper_connection,
            // AI Configuration
            configure_ai_provider,
            // Conversation Management
            create_conversation,
            list_conversations,
            get_conversation,
            delete_conversation,
            clear_conversation,
            // Messaging
            send_message,
            // Encyclopedia
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
