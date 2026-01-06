//! AI Chain Orchestrator (Act mode helper)
//!
//! This module helps the AI take stronger control over the FX chain by:
//! - providing installed plugin catalog context
//! - running multi-pass planning (phase1 can load/reorder; phase2 refines without loads)

use crate::ai_client::AIProvider;
use crate::parameter_ai::{ParameterAI, ParameterAIOptions, ParameterAIResult, ParameterAction, ReaperSnapshot};
use crate::reaper_client::ReaperClient;
use crate::tone_encyclopedia::ToneParameters;
use serde_json::json;

use crate::act_mode::{ActProgressEvent, ActProgressSink};

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub include_catalog_in_prompt: bool,
    pub catalog_names_limit: usize,
    pub phase1_max_actions: usize,
    pub phase2_max_actions: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            include_catalog_in_prompt: true,
            catalog_names_limit: 250,
            phase1_max_actions: 220,
            phase2_max_actions: 260,
        }
    }
}

pub struct AIChainOrchestrator {
    reaper: ReaperClient,
    ai: AIProvider,
    config: OrchestratorConfig,
}

impl AIChainOrchestrator {
    pub fn new(reaper: ReaperClient, ai: AIProvider, config: OrchestratorConfig) -> Self {
        Self { reaper, ai, config }
    }

    pub async fn plan_phase1(
        &self,
        tone_params: &ToneParameters,
        snapshot: &ReaperSnapshot,
        tone_description: &str,
        user_message: &str,
        progress: Option<&dyn ActProgressSink>,
    ) -> Result<(ParameterAIResult, bool), String> {
        let parameter_ai = ParameterAI::new(self.ai.clone());

        let mut extra = String::new();
        extra.push_str("Build a high-quality, modern FX chain. You may load plugins and reorder the chain.\n");
        extra.push_str("You may use move_plugin to improve signal flow (e.g., gate->EQ->drive->amp->cab->postEQ->space).\n");
        extra.push_str("Prefer sensible gain staging and avoid extreme wet mixes unless explicitly requested.\n");
        extra.push_str("If you include any load_plugin actions, do NOT set parameters on newly loaded plugins in phase1.\n");

        if self.config.include_catalog_in_prompt {
            if let Ok(catalog) = self.reaper.get_fx_catalog(false).await {
                if let Some(plugins) = catalog.get("plugins").and_then(|v| v.as_array()) {
                    extra.push_str("\n=== INSTALLED FX CATALOG (names only) ===\n");
                    for p in plugins.iter().take(self.config.catalog_names_limit) {
                        if let Some(name) = p.get("name").and_then(|v| v.as_str()) {
                            extra.push_str("- ");
                            extra.push_str(name);
                            extra.push('\n');
                        }
                    }
                }
            }
        }

        emit(
            progress,
            ActProgressEvent {
                stage: "map".to_string(),
                level: "info".to_string(),
                message: "AI planning phase1 (may load/reorder)".to_string(),
                details: Some(json!({"max_actions": self.config.phase1_max_actions})),
                step: None,
            },
        );

        let phase1_opts = ParameterAIOptions {
            allow_load_plugins: true,
            max_actions: self.config.phase1_max_actions,
            phase_name: "phase1".to_string(),
        };
        let phase1 = parameter_ai
            .map_parameters_with_options(tone_params, snapshot, tone_description, &phase1_opts, Some(&extra))
            .await
            .map_err(|e| format!("Parameter AI phase1 error: {}", e))?;

        let requires_resnapshot = phase1.actions.iter().any(|a| {
            matches!(a, ParameterAction::LoadPlugin { .. } | ParameterAction::MovePlugin { .. })
        });

        if requires_resnapshot {
            emit(
                progress,
                ActProgressEvent {
                    stage: "map".to_string(),
                    level: "info".to_string(),
                    message: "Phase1 included load/move actions; will resnapshot and run phase2".to_string(),
                    details: Some(json!({"actions": phase1.actions.len(), "request": user_message})),
                    step: None,
                },
            );
        }

        Ok((phase1, requires_resnapshot))
    }

    pub async fn plan_phase2(
        &self,
        tone_params: &ToneParameters,
        snapshot: &ReaperSnapshot,
        tone_description: &str,
        progress: Option<&dyn ActProgressSink>,
    ) -> Result<ParameterAIResult, String> {
        let parameter_ai = ParameterAI::new(self.ai.clone());

        emit(
            progress,
            ActProgressEvent {
                stage: "map".to_string(),
                level: "info".to_string(),
                message: "AI planning phase2 (no loads, refine chain/params)".to_string(),
                details: Some(json!({"max_actions": self.config.phase2_max_actions})),
                step: None,
            },
        );

        let phase2_opts = ParameterAIOptions {
            allow_load_plugins: false,
            max_actions: self.config.phase2_max_actions,
            phase_name: "phase2".to_string(),
        };

        parameter_ai
            .map_parameters_with_options(
                tone_params,
                snapshot,
                tone_description,
                &phase2_opts,
                Some("Do not load plugins in phase2. Refine parameters and order only."),
            )
            .await
            .map_err(|e| format!("Parameter AI phase2 error: {}", e))
    }
}

fn emit(sink: Option<&dyn ActProgressSink>, event: ActProgressEvent) {
    let Some(sink) = sink else { return };
    sink.emit(event);
}

