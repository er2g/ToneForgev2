//! Act Mode - Direct Application (Two-Tier System)
//!
//! This mode applies tones directly to REAPER using the two-tier AI system:
//! - Tier 1 (Tone AI): Search encyclopedia or generate tone
//! - Tier 2 (Parameter AI): Map parameters to REAPER and apply
//!
//! FULL REAPER access - applies changes!

use crate::ai_client::AIProvider;
use crate::chain_mapper::{ChainMapper, ChainMapperConfig};
use crate::parameter_ai::{ParameterAction, ReaperParameter, ReaperPlugin, ReaperSnapshot};
use crate::reaper_client::ReaperClient;
use crate::tone_ai::ToneAI;
use crate::tone_sanitizer;
use crate::tone_encyclopedia::ToneEncyclopedia;
use crate::undo_redo::UndoManager;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Clone, Serialize)]
pub struct ProgressStep {
    pub current: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActProgressEvent {
    pub stage: String,
    pub level: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<ProgressStep>,
}

pub trait ActProgressSink: Send + Sync {
    fn emit(&self, event: ActProgressEvent);
}

/// Act mode handler
pub struct ActMode {
    encyclopedia: ToneEncyclopedia,
    reaper_client: ReaperClient,
    ai_provider: AIProvider,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActResponse {
    pub tone_source: String,
    pub tone_description: String,
    pub confidence: f32,
    pub summary: String,
    pub actions_count: usize,
    pub action_logs: Vec<String>,
    pub warnings: Vec<String>,
}

impl ActMode {
    /// Create new act mode handler
    pub fn new(
        encyclopedia: ToneEncyclopedia,
        reaper_client: ReaperClient,
        ai_provider: AIProvider,
    ) -> Self {
        Self {
            encyclopedia,
            reaper_client,
            ai_provider,
        }
    }

    /// Process an action request (apply tone to REAPER)
    pub async fn process_message(
        &self,
        user_message: &str,
        track_index: i32,
        undo_manager: &mut UndoManager,
    ) -> Result<ActResponse, String> {
        self.process_message_with_progress(user_message, track_index, undo_manager, None)
            .await
    }

    pub async fn process_message_with_progress(
        &self,
        user_message: &str,
        track_index: i32,
        undo_manager: &mut UndoManager,
        progress: Option<&dyn ActProgressSink>,
    ) -> Result<ActResponse, String> {
        println!("\n========== ACT MODE: TWO-TIER AI PIPELINE ==========");
        println!("[USER] {}", user_message);
        emit(progress, "start", "info", "Act mode pipeline started", None, None);

        // ========== TIER 1: TONE AI ==========
        println!("\n[TIER 1] Running Tone AI...");
        emit(
            progress,
            "tone_ai",
            "info",
            "Running Tone AI (encyclopedia search + AI fallback if needed)",
            None,
            None,
        );

        let tone_ai = ToneAI::new(self.encyclopedia.clone())
            .with_ai_provider(self.ai_provider.clone());

        let tone_result = tone_ai
            .process_request(user_message)
            .await
            .map_err(|e| format!("Tone AI error: {}", e))?;

        println!("[TIER 1] Result:");
        println!("  - Source: {:?}", tone_result.source);
        println!("  - Description: {}", tone_result.tone_description);
        println!("  - Confidence: {:.0}%", tone_result.confidence * 100.0);
        emit(
            progress,
            "tone_ai",
            "info",
            "Tone AI produced parameters",
            Some(json!({
                "source": format!("{:?}", tone_result.source),
                "confidence": tone_result.confidence,
                "description": tone_result.tone_description,
                "matched_entry": tone_result.matched_entry,
            })),
            None,
        );

        let sanitized = tone_sanitizer::sanitize(tone_result.parameters.clone());
        let tone_params = sanitized.parameters;
        let mut tone_warnings = sanitized.warnings;
        if !tone_warnings.is_empty() {
            emit(
                progress,
                "sanitize",
                "warn",
                "Sanitizer produced warnings",
                Some(json!({ "warnings": tone_warnings })),
                None,
            );
        } else {
            emit(progress, "sanitize", "info", "Sanitizer ok", None, None);
        }

        // ========== GET REAPER SNAPSHOT ==========
        println!("\n[REAPER] Fetching current state...");
        emit(progress, "snapshot", "info", "Fetching REAPER track/FX snapshot", None, None);

        let reaper_snapshot = self
            .collect_reaper_snapshot(track_index)
            .await
            .map_err(|e| format!("Failed to get REAPER state: {}", e))?;

        println!("[REAPER] Track: {}", reaper_snapshot.track_name);
        println!("[REAPER] Plugins: {}", reaper_snapshot.plugins.len());
        emit(
            progress,
            "snapshot",
            "info",
            "Snapshot collected",
            Some(json!({
                "track_index": reaper_snapshot.track_index,
                "track_name": reaper_snapshot.track_name,
                "plugins": reaper_snapshot.plugins.iter().map(|p| json!({"index": p.index, "name": p.name, "enabled": p.enabled, "param_count": p.parameters.len()})).collect::<Vec<_>>(),
            })),
            None,
        );

        // ========== TIER 2: DETERMINISTIC CHAIN MAPPER ==========
        println!("\n[TIER 2] Running Deterministic Chain Mapper...");
        emit(progress, "map", "info", "Mapping tone parameters to deterministic actions", None, None);

        let mapper = ChainMapper::new(ChainMapperConfig::default());
        let mut mapping = mapper.map(&tone_params, &reaper_snapshot);
        emit(
            progress,
            "map",
            "info",
            "Mapper produced action plan",
            Some(json!({
                "summary": mapping.summary,
                "actions": mapping.actions.len(),
                "requires_resnapshot": mapping.requires_resnapshot,
                "warnings": mapping.warnings,
            })),
            None,
        );

        // If mapper decided to load plugins, apply load actions first, re-snapshot, then remap once.
        if mapping.requires_resnapshot {
            println!("[TIER 2] Applying load actions and refreshing REAPER snapshot...");
            emit(
                progress,
                "apply",
                "info",
                "Applying load/enable prerequisites (requires resnapshot)",
                None,
                None,
            );

            let load_actions: Vec<ParameterAction> = mapping
                .actions
                .iter()
                .cloned()
                .filter(|a| matches!(a, ParameterAction::LoadPlugin { .. } | ParameterAction::EnablePlugin { .. }))
                .collect();

            // Begin undo group early to keep a single action label.
            undo_manager.begin_action(&format!("Tone: {}", user_message));
            let load_result = self
                .apply_parameter_actions(&load_actions, &reaper_snapshot, undo_manager, progress)
                .await
                .map_err(|e| format!("Failed to apply load actions: {}", e))?;

            let refreshed = self
                .collect_reaper_snapshot(track_index)
                .await
                .map_err(|e| format!("Failed to refresh REAPER state: {}", e))?;
            emit(
                progress,
                "snapshot",
                "info",
                "Snapshot refreshed after loads/enables",
                Some(json!({
                    "track_index": refreshed.track_index,
                    "track_name": refreshed.track_name,
                    "plugins": refreshed.plugins.len(),
                })),
                None,
            );

            let mapper_no_load = ChainMapper::new(ChainMapperConfig {
                allow_load_plugins: false,
                ..Default::default()
            });
            mapping = mapper_no_load.map(&tone_params, &refreshed);
            emit(
                progress,
                "map",
                "info",
                "Remapped after resnapshot",
                Some(json!({
                    "summary": mapping.summary,
                    "actions": mapping.actions.len(),
                    "warnings": mapping.warnings,
                })),
                None,
            );

            let mut apply_result = self
                .apply_parameter_actions(&mapping.actions, &refreshed, undo_manager, progress)
                .await
                .map_err(|e| format!("Failed to apply actions: {}", e))?;

            // Keep a full log for transparency (loads/enables first, then sets).
            apply_result.logs.splice(0..0, load_result.logs);
            apply_result.warnings.splice(0..0, load_result.warnings);

            tone_warnings.extend(apply_result.warnings);

            if let Some(action_id) = undo_manager.commit_action() {
                println!("[UNDO] Recorded action: {}", action_id);
            }

            println!("\n========== ACT MODE: PIPELINE COMPLETE ==========\n");
            emit(progress, "done", "info", "Act mode pipeline complete", None, None);

            return Ok(ActResponse {
                tone_source: format!("{:?}", tone_result.source),
                tone_description: tone_result.tone_description,
                confidence: tone_result.confidence,
                summary: mapping.summary,
                actions_count: mapping.actions.len(),
                action_logs: apply_result.logs,
                warnings: {
                    let mut w = mapping.warnings;
                    w.extend(tone_warnings);
                    w
                },
            });
        }

        println!("[TIER 2] Generated {} actions", mapping.actions.len());
        println!("[TIER 2] Summary: {}", mapping.summary);

        let mut all_warnings = mapping.warnings.clone();
        all_warnings.extend(tone_warnings.clone());

        if !all_warnings.is_empty() {
            println!("\n[VALIDATION] Warnings:");
            for warning in &all_warnings {
                println!("  ⚠️  {}", warning);
            }
        }

        // ========== RECORD FOR UNDO ==========
        undo_manager.begin_action(&format!("Tone: {}", user_message));

        // ========== APPLY ACTIONS TO REAPER ==========
        println!("\n[APPLY] Applying actions to REAPER...");
        emit(
            progress,
            "apply",
            "info",
            "Applying actions to REAPER",
            Some(json!({ "actions": mapping.actions.len() })),
            None,
        );

        let apply_result = self
            .apply_parameter_actions(&mapping.actions, &reaper_snapshot, undo_manager, progress)
            .await
            .map_err(|e| format!("Failed to apply actions: {}", e))?;

        all_warnings.extend(apply_result.warnings.clone());

        for log in &apply_result.logs {
            println!("[ACTION] {}", log);
        }

        // ========== COMMIT UNDO ==========
        if let Some(action_id) = undo_manager.commit_action() {
            println!("[UNDO] Recorded action: {}", action_id);
        }

        println!("\n========== ACT MODE: PIPELINE COMPLETE ==========\n");
        emit(progress, "done", "info", "Act mode pipeline complete", None, None);

        Ok(ActResponse {
            tone_source: format!("{:?}", tone_result.source),
            tone_description: tone_result.tone_description,
            confidence: tone_result.confidence,
            summary: mapping.summary,
            actions_count: mapping.actions.len(),
            action_logs: apply_result.logs,
            warnings: all_warnings,
        })
    }

    async fn collect_reaper_snapshot(
        &self,
        track_idx: i32,
    ) -> Result<ReaperSnapshot, Box<dyn Error>> {
        let overview = self.reaper_client.get_tracks().await?;

        let track = overview
            .tracks
            .iter()
            .find(|t| t.index == track_idx)
            .ok_or_else(|| format!("Track {} not found", track_idx))?;

        let mut plugins = Vec::new();

        for fx in &track.fx_list {
            let params_snapshot = self.reaper_client.get_fx_params(track_idx, fx.index).await?;

            let parameters: Vec<ReaperParameter> = params_snapshot
                .params
                .into_iter()
                .map(|p| ReaperParameter {
                    index: p.index,
                    name: p.name,
                    current_value: p.value,
                    display_value: p.display,
                    unit: p.unit,
                    format_hint: p.format_hint,
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

    async fn apply_parameter_actions(
        &self,
        actions: &[ParameterAction],
        snapshot: &ReaperSnapshot,
        undo_manager: &mut UndoManager,
        progress: Option<&dyn ActProgressSink>,
    ) -> Result<ApplyResult, Box<dyn Error>> {
        let mut logs = Vec::new();
        let mut warnings = Vec::new();

        for (idx, action) in actions.iter().enumerate() {
            match action {
                ParameterAction::SetParameter {
                    track,
                    plugin_index,
                    param_index,
                    param_name,
                    value,
                    reason,
                } => {
                    if let Some(plugin) = snapshot.plugins.iter().find(|p| p.index == *plugin_index) {
                        if let Some(param) = plugin.parameters.iter().find(|p| p.index == *param_index) {
                            // Record for undo
                            undo_manager.record_param_change(
                                *track,
                                *plugin_index,
                                &plugin.name,
                                *param_index,
                                param_name,
                                param.current_value,
                                *value,
                            );

                            // Apply change
                            self.reaper_client
                                .set_param_by_index(*track, *plugin_index, *param_index, *value)
                                .await?;

                            // Verify (best-effort): read back normalized value
                            if let Ok(applied) = self
                                .reaper_client
                                .get_param_by_index(*track, *plugin_index, *param_index)
                                .await
                            {
                                if (applied - *value).abs() > 0.02 {
                                    let w = format!(
                                        "Param verify mismatch: {} :: {} expected {:.3} got {:.3}",
                                        plugin.name, param_name, value, applied
                                    );
                                    warnings.push(w.clone());
                                    emit(
                                        progress,
                                        "verify",
                                        "warn",
                                        &w,
                                        Some(json!({
                                            "plugin": plugin.name,
                                            "param": param_name,
                                            "expected": value,
                                            "applied": applied,
                                        })),
                                        Some(ProgressStep {
                                            current: idx + 1,
                                            total: actions.len(),
                                        }),
                                    );
                                }
                            }

                            emit(
                                progress,
                                "apply",
                                "info",
                                "Set parameter",
                                Some(json!({
                                    "plugin": plugin.name,
                                    "param": param_name,
                                    "old": param.current_value,
                                    "new": value,
                                    "reason": reason,
                                })),
                                Some(ProgressStep {
                                    current: idx + 1,
                                    total: actions.len(),
                                }),
                            );

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
                    if let Some(plugin) = snapshot.plugins.iter().find(|p| p.index == *plugin_index) {
                        undo_manager.record_fx_toggle(*track, *plugin_index, plugin_name, plugin.enabled);
                    }

                    self.reaper_client.set_fx_enabled(*track, *plugin_index, true).await?;
                    emit(
                        progress,
                        "apply",
                        "info",
                        "Enabled plugin",
                        Some(json!({
                            "plugin": plugin_name,
                            "reason": reason,
                        })),
                        Some(ProgressStep {
                            current: idx + 1,
                            total: actions.len(),
                        }),
                    );

                    logs.push(format!("✓ Enabled '{}' - {}", plugin_name, reason));
                }
                ParameterAction::LoadPlugin {
                    track,
                    plugin_name,
                    reason,
                    ..
                } => {
                    let slot = self.reaper_client.add_plugin(*track, plugin_name).await?;
                    emit(
                        progress,
                        "apply",
                        "info",
                        "Loaded plugin",
                        Some(json!({
                            "plugin": plugin_name,
                            "slot": slot,
                            "reason": reason,
                        })),
                        Some(ProgressStep {
                            current: idx + 1,
                            total: actions.len(),
                        }),
                    );

                    logs.push(format!(
                        "✓ Loaded '{}' at slot {} - {}",
                        plugin_name, slot, reason
                    ));
                }
            }
        }

        Ok(ApplyResult { logs, warnings })
    }
}

struct ApplyResult {
    logs: Vec<String>,
    warnings: Vec<String>,
}

fn emit(
    sink: Option<&dyn ActProgressSink>,
    stage: &str,
    level: &str,
    message: &str,
    details: Option<Value>,
    step: Option<ProgressStep>,
) {
    let Some(sink) = sink else { return };
    sink.emit(ActProgressEvent {
        stage: stage.to_string(),
        level: level.to_string(),
        message: message.to_string(),
        details,
        step,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_act_mode_creation() {
        let encyclopedia = ToneEncyclopedia::new();
        let reaper = ReaperClient::new();
        let provider = crate::ai_client::AIProvider::grok("test".to_string(), "test".to_string());

        let _act_mode = ActMode::new(encyclopedia, reaper, provider);
    }
}
