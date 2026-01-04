use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use toneforge_mapper_tests::parameter_ai::{ParameterAction, ReaperParameter, ReaperPlugin, ReaperSnapshot};
use toneforge_mapper_tests::tone_encyclopedia::{EffectParameters, ToneParameters};
use toneforge_mapper_tests::{sanitize_tone, ChainMapper, ChainMapperConfig};

const BASE_URL: &str = "http://127.0.0.1:8888";
const NO_EFFECTS: &[&str] = &[];
const NEED_CHORUS: &[&str] = &["chorus"];
const NEED_DISTORTION: &[&str] = &["distortion"];
const NEED_COMPRESSOR: &[&str] = &["compressor"];
const NEED_OVERDRIVE: &[&str] = &["overdrive"];
const FORBID_DIST_OR_OD: &[&str] = &["distortion", "overdrive"];
const TONE_ENGINEER_SYSTEM_PROMPT: &str = r#"You are a professional guitar/bass tone specialist AI.
Your job is to analyze tone requests and provide precise parameter recommendations.

When given a tone request, respond with a JSON object containing:
1. A description of the tone
2. Precise parameter values (0.0 to 1.0 range)

REQUIREMENTS:
- If the user explicitly asks for an effect category (delay, reverb, noise gate, EQ), include that section with at least one meaningful parameter.
- If the user explicitly says to keep that category OFF/bypassed, leave that section empty (or omit it).
- Do NOT add extra sections or effects that the user did not ask for.

Example output:
```json
{
  "description": "Aggressive thrash metal tone with scooped mids",
  "parameters": {
    "amp": {
      "gain": 0.85,
      "bass": 0.6,
      "mid": 0.3,
      "treble": 0.75,
      "presence": 0.7
    },
    "eq": {
      "80Hz": -2.0,
      "800Hz": -4.0,
      "2kHz": 1.0,
      "5kHz": 2.0
    },
    "effects": [
      {
        "effect_type": "noise_gate",
        "parameters": {"threshold": 0.3}
      },
      {
        "effect_type": "overdrive",
        "parameters": {"drive": 0.4, "tone": 0.6, "level": 0.8}
      }
    ],
    "reverb": {
      "room_size": 0.2,
      "mix": 0.1
    }
  }
}
```

IMPORTANT:
- All amp/effect parameters must be normalized to 0.0-1.0 range
- EQ values are in dB (-12.0 to +12.0)
- Be precise and consistent
- Respond ONLY with valid JSON
"#;

#[tokio::main]
async fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--help" || a == "-h") {
        println!(
            r#"Usage: gemini_chain_test [--offline] [--api-key <KEY>] [--api-key-path <PATH>]

API key resolution order:
  1) --api-key <KEY>
  2) Env: API_KEY, GEMINI_API_KEY, VERTEX_API_KEY
  3) --api-key-path <PATH>
  4) Fallback files: api.txt, ../api.txt, ../../api.txt, ../../../api.txt
"#
        );
        return Ok(());
    }

    let credential = resolve_api_key()?;
    let model = "gemini-2.5-pro";
    let offline = std::env::args().any(|a| a == "--offline");

    let mut server = start_mock_server()?;
    wait_for_ping().await?;

    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("build http client")?;

    let tests = vec![
        TestCase {
            name: "Delay bypassed section",
            scenario: "confusing_delay_section",
            prompt: "Tight modern rhythm. Add a subtle slapback delay (short time), low mix, low feedback. Keep it tight and do not wash out the tone.",
            expect: Expectations {
                require_delay: true,
                require_reverb: false,
                require_gate: false,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: Some(0.03),
                max_delay_mix: Some(0.20),
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Reverb bypassed section",
            scenario: "bypassed_reverb",
            prompt: "Add a very subtle small room reverb: low mix, small room size. Keep clarity and avoid washing out the attack.",
            expect: Expectations {
                require_delay: false,
                require_reverb: true,
                require_gate: false,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: Some(0.02),
                max_reverb_mix: Some(0.15),
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Gate plugin disabled",
            scenario: "disabled_gate",
            prompt: "Very tight high-gain metal rhythm. Add a noise gate that closes fast between chugs. Avoid pumping.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: true,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Gate section disabled",
            scenario: "gate_enable_off",
            prompt: "Very tight high-gain metal rhythm. Use a noise gate: lower threshold a bit, fast attack, medium release. Make sure the gate is actually ON.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: true,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Reverb missing (should load)",
            scenario: "missing_reverb",
            prompt: "Add a small room reverb with very low mix, just a little space. Keep clarity.",
            expect: Expectations {
                require_delay: false,
                require_reverb: true,
                require_gate: false,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: Some(0.01),
                max_reverb_mix: Some(0.12),
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "EQ bypassed section",
            scenario: "bypassed_eq",
            prompt: "Scoop some mud around 250 Hz, keep tight lows, add a bit of presence. Use EQ to cut ~250Hz a few dB.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: false,
                require_eq: true,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Kitchen sink contradictions",
            scenario: "kitchen_sink",
            prompt: "Make it extreme modern djent: very tight gate, some delay, tiny room reverb, scoop mud around 250Hz, add presence for attack. BUT keep reverb OFF and keep delay bypassed. Avoid clipping at all costs.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: true,
                require_eq: true,
                forbid_delay: true,
                forbid_reverb: true,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Dual delay prefer ReaDelay",
            scenario: "dual_delay_prefer_readelay",
            prompt: "Add a subtle slapback delay. Use short time, low mix, low feedback. Keep it tight.",
            expect: Expectations {
                require_delay: true,
                require_reverb: false,
                require_gate: false,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: Some(0.03),
                max_delay_mix: Some(0.20),
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: Some("ReaDelay"),
        },
        TestCase {
            name: "Shoegaze wall (niche)",
            scenario: "shoegaze_wall",
            prompt: "Create an MBV-style shoegaze wall: modulated chorus, dotted-eighth delay, and big ambient reverb. No noise gate. Keep it musical, not sterile.",
            expect: Expectations {
                require_delay: true,
                require_reverb: true,
                require_gate: false,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: false,
                forbid_gate: true,
                forbid_eq: false,
                min_delay_mix: Some(0.18),
                max_delay_mix: None,
                min_reverb_mix: Some(0.22),
                max_reverb_mix: None,
            },
            required_effects: NEED_CHORUS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: Some("ReaDelay"),
        },
        TestCase {
            name: "Swedish chainsaw (niche)",
            scenario: "chainsaw_distortion_bypassed",
            prompt: "Swedish chainsaw HM-2 style: brutal distortion, max low/high, and scoop low-mids for clarity. No delay, no reverb.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: false,
                require_eq: true,
                forbid_delay: true,
                forbid_reverb: true,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NEED_DISTORTION,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Funk compressor (niche)",
            scenario: "funk_compressor_disabled",
            prompt: "Nile Rodgers clean funk: add compression for tightness (moderate ratio, medium attack, fast-ish release). No delay, no reverb. Avoid distortion/overdrive.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: false,
                require_eq: false,
                forbid_delay: true,
                forbid_reverb: true,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NEED_COMPRESSOR,
            forbidden_effects: FORBID_DIST_OR_OD,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Tubescreamer tighten (niche)",
            scenario: "overdrive_bypassed",
            prompt: "Tighten the low end with a tubescreamer-style overdrive: low drive, higher level, slightly bright tone. No delay, no reverb.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: false,
                require_eq: false,
                forbid_delay: true,
                forbid_reverb: true,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NEED_OVERDRIVE,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
        TestCase {
            name: "Contradiction: keep reverb OFF",
            scenario: "bypassed_reverb",
            prompt: "Add a huge reverb but keep the reverb OFF/bypassed (do not apply reverb). Focus on core tone instead.",
            expect: Expectations {
                require_delay: false,
                require_reverb: false,
                require_gate: false,
                require_eq: false,
                forbid_delay: false,
                forbid_reverb: true,
                forbid_gate: false,
                forbid_eq: false,
                min_delay_mix: None,
                max_delay_mix: None,
                min_reverb_mix: None,
                max_reverb_mix: None,
            },
            required_effects: NO_EFFECTS,
            forbidden_effects: NO_EFFECTS,
            prefer_delay_plugin_contains: None,
        },
    ];

    let mut reports = Vec::new();

    for t in tests {
        reset_scenario(&client, t.scenario).await?;

        let mut online_error: Option<String> = None;
        let (mut engineer_out, mut engine_label) = if offline {
            (offline_engineer(t.name, t.prompt), "offline")
        } else {
            match gemini_tone_engineer(&client, &credential, model, t.prompt).await {
                Ok(v) => (v, "vertex-gemini"),
                Err(e) => {
                    online_error = Some(format!("{e}"));
                    (offline_engineer(t.name, t.prompt), "offline-fallback")
                }
            }
        };

        // Evaluate and optionally attempt a single repair pass (online mode only).
        let mut sanitized = sanitize_tone(engineer_out.parameters.clone());
        let mut sanitize_warnings = sanitized.warnings.clone();
        let mut engine_eval = evaluate_engineer(
            &sanitized.parameters,
            &t.expect,
            t.required_effects,
            t.forbidden_effects,
        );

        if !offline && online_error.is_none() && !engine_eval.ok {
            match gemini_tone_engineer_repair(
                &client,
                &credential,
                model,
                t.prompt,
                &engineer_out.description,
                &sanitized.parameters,
                &engine_eval.warnings,
            )
            .await
            {
                Ok(repaired) => {
                    engineer_out = repaired;
                    engine_label = "vertex-gemini+repair";
                    sanitized = sanitize_tone(engineer_out.parameters.clone());
                    sanitize_warnings = sanitized.warnings.clone();
                    engine_eval = evaluate_engineer(
                        &sanitized.parameters,
                        &t.expect,
                        t.required_effects,
                        t.forbidden_effects,
                    );
                }
                Err(e) => {
                    // Keep original output, but record the repair failure.
                    let msg = format!("repair_failed: {e}");
                    online_error = Some(match online_error {
                        Some(prev) => format!("{prev}; {msg}"),
                        None => msg,
                    });
                }
            }
        }

        // Apply-side pruning: keep only requested/allowed sections to avoid "distracting" the applier.
        let (apply_params, prune_warnings) = prune_for_apply(&sanitized.parameters, &t);

        let snapshot = collect_snapshot(&client, 0).await?;
        let mapper = ChainMapper::new(ChainMapperConfig::default());
        let mut mapping = mapper.map(&apply_params, &snapshot);

        let mut apply_warnings = Vec::new();
        let mut action_logs = Vec::new();

        if mapping.requires_resnapshot {
            let load_actions: Vec<ParameterAction> = mapping
                .actions
                .iter()
                .cloned()
                .filter(|a| matches!(a, ParameterAction::LoadPlugin { .. } | ParameterAction::EnablePlugin { .. }))
                .collect();
            let load_res = apply_actions(&client, &load_actions).await?;
            action_logs.extend(load_res.logs);
            apply_warnings.extend(load_res.warnings);

            let refreshed = collect_snapshot(&client, 0).await?;
            let mapper_no_load = ChainMapper::new(ChainMapperConfig {
                allow_load_plugins: false,
                ..Default::default()
            });
            mapping = mapper_no_load.map(&apply_params, &refreshed);
            let apply_res = apply_actions(&client, &mapping.actions).await?;
            action_logs.extend(apply_res.logs);
            apply_warnings.extend(apply_res.warnings);

            // re-collect for invariant checks
            let final_snapshot = collect_snapshot(&client, 0).await?;
            let invariants = check_invariants(&refreshed, &final_snapshot, &mapping.actions);
            let mut mapping_warnings = mapping.warnings.clone();
            mapping_warnings.extend(prune_warnings.clone());
            mapping_warnings.extend(evaluate_mapping(&refreshed, &mapping.actions, &t));

            reports.push(Report::ok(
                t.name,
                t.scenario,
                engine_label,
                engineer_out.description,
                engine_eval,
                sanitize_warnings,
                mapping_warnings,
                apply_warnings,
                action_logs,
                invariants,
                online_error,
            ));
        } else {
            let apply_res = apply_actions(&client, &mapping.actions).await?;
            action_logs.extend(apply_res.logs);
            apply_warnings.extend(apply_res.warnings);
            let final_snapshot = collect_snapshot(&client, 0).await?;
            let invariants = check_invariants(&snapshot, &final_snapshot, &mapping.actions);
            let mut mapping_warnings = mapping.warnings.clone();
            mapping_warnings.extend(prune_warnings);
            mapping_warnings.extend(evaluate_mapping(&snapshot, &mapping.actions, &t));

            reports.push(Report::ok(
                t.name,
                t.scenario,
                engine_label,
                engineer_out.description,
                engine_eval,
                sanitize_warnings,
                mapping_warnings,
                apply_warnings,
                action_logs,
                invariants,
                online_error,
            ));
        }
    }

    // Stop server
    let _ = server.kill();

    print_report(&reports);
    Ok(())
}

struct TestCase<'a> {
    name: &'a str,
    scenario: &'a str,
    prompt: &'a str,
    expect: Expectations,
    required_effects: &'a [&'a str],
    forbidden_effects: &'a [&'a str],
    prefer_delay_plugin_contains: Option<&'a str>,
}

#[derive(Clone, Copy)]
struct Expectations {
    require_delay: bool,
    require_reverb: bool,
    require_gate: bool,
    require_eq: bool,
    forbid_delay: bool,
    forbid_reverb: bool,
    forbid_gate: bool,
    forbid_eq: bool,
    min_delay_mix: Option<f64>,
    max_delay_mix: Option<f64>,
    min_reverb_mix: Option<f64>,
    max_reverb_mix: Option<f64>,
}

struct EngineerEval {
    ok: bool,
    score: i32,
    warnings: Vec<String>,
}

struct EngineerOut {
    description: String,
    parameters: ToneParameters,
}

struct ApplyRes {
    logs: Vec<String>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct Invariants {
    enable_action_before_set: bool,
    delay_bypass_cleared_if_delay_set: bool,
    gate_enable_cleared_if_threshold_set: bool,
    reverb_bypass_cleared_if_reverb_set: bool,
    eq_bypass_cleared_if_eq_set: bool,
    plugins_enabled_if_params_set: bool,
    no_param_changes_while_inactive: bool,
}

struct Report {
    name: String,
    scenario: String,
    ok: bool,
    description: Option<String>,
    error: Option<String>,
    engine: Option<String>,
    online_error: Option<String>,
    engineer_warnings: Vec<String>,
    engineer_score: i32,
    sanitize_warnings: Vec<String>,
    mapping_warnings: Vec<String>,
    apply_warnings: Vec<String>,
    logs: Vec<String>,
    invariants: Option<Invariants>,
}

impl Report {
    fn ok(
        name: &str,
        scenario: &str,
        engine: &str,
        description: String,
        engineer_eval: EngineerEval,
        sanitize_warnings: Vec<String>,
        mapping_warnings: Vec<String>,
        apply_warnings: Vec<String>,
        logs: Vec<String>,
        invariants: Invariants,
        online_error: Option<String>,
    ) -> Self {
        let ok = engineer_eval.ok
            && invariants.enable_action_before_set
            && invariants.plugins_enabled_if_params_set
            && invariants.no_param_changes_while_inactive
            && invariants.delay_bypass_cleared_if_delay_set
            && invariants.gate_enable_cleared_if_threshold_set
            && invariants.reverb_bypass_cleared_if_reverb_set
            && invariants.eq_bypass_cleared_if_eq_set;
        Self {
            name: name.to_string(),
            scenario: scenario.to_string(),
            ok,
            description: Some(description),
            error: None,
            engine: Some(engine.to_string()),
            online_error,
            engineer_warnings: engineer_eval.warnings,
            engineer_score: engineer_eval.score,
            sanitize_warnings,
            mapping_warnings,
            apply_warnings,
            logs,
            invariants: Some(invariants),
        }
    }
}

fn print_report(reports: &[Report]) {
    println!("=== Gemini Chain Test Report ===");
    for r in reports {
        println!();
        println!(
            "- {} [{}] => {}{}",
            r.name,
            r.scenario,
            if r.ok { "PASS" } else { "FAIL" },
            r.engine
                .as_ref()
                .map(|e| format!(" ({})", e))
                .unwrap_or_default()
        );
        if let Some(e) = &r.error {
            println!("  error: {}", e);
            continue;
        }
        if let Some(e) = &r.online_error {
            println!("  online_error: {}", summarize(e, 180));
        }
        if let Some(d) = &r.description {
            println!("  tone: {}", summarize(d, 120));
        }
        println!(
            "  engineer_score: {} (warnings: {})",
            r.engineer_score,
            r.engineer_warnings.len()
        );
        if !r.engineer_warnings.is_empty() {
            for w in r.engineer_warnings.iter().take(3) {
                println!("    - {}", summarize(w, 140));
            }
        }
        if !r.sanitize_warnings.is_empty() {
            println!("  sanitize_warnings: {}", r.sanitize_warnings.len());
            for w in r.sanitize_warnings.iter().take(3) {
                println!("    - {}", summarize(w, 140));
            }
        }
        if !r.mapping_warnings.is_empty() {
            println!("  mapping_warnings: {}", r.mapping_warnings.len());
            for w in r.mapping_warnings.iter().take(3) {
                println!("    - {}", summarize(w, 140));
            }
        }
        if !r.apply_warnings.is_empty() {
            println!("  apply_warnings: {}", r.apply_warnings.len());
            for w in r.apply_warnings.iter().take(3) {
                println!("    - {}", summarize(w, 140));
            }
        }
        if let Some(inv) = &r.invariants {
            println!("  invariants:");
            println!("    - enable_action_before_set: {}", inv.enable_action_before_set);
            println!(
                "    - plugins_enabled_if_params_set: {}",
                inv.plugins_enabled_if_params_set
            );
            println!(
                "    - no_param_changes_while_inactive: {}",
                inv.no_param_changes_while_inactive
            );
            println!(
                "    - delay_bypass_cleared_if_delay_set: {}",
                inv.delay_bypass_cleared_if_delay_set
            );
            println!(
                "    - gate_enable_cleared_if_threshold_set: {}",
                inv.gate_enable_cleared_if_threshold_set
            );
            println!(
                "    - reverb_bypass_cleared_if_reverb_set: {}",
                inv.reverb_bypass_cleared_if_reverb_set
            );
            println!(
                "    - eq_bypass_cleared_if_eq_set: {}",
                inv.eq_bypass_cleared_if_eq_set
            );
        }
        // show a tiny log sample for debugging
        for l in r.logs.iter().take(3) {
            println!("  log: {}", summarize(l, 160));
        }
    }
}

fn summarize(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut out = s[..max].to_string();
    out.push_str("…");
    out
}

fn evaluate_engineer(
    tone: &ToneParameters,
    expect: &Expectations,
    required_effects: &[&str],
    forbidden_effects: &[&str],
) -> EngineerEval {
    fn norm(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    let mut score: i32 = 100;
    let mut warnings = Vec::new();
    let mut hard_fail = false;

    if expect.require_delay && tone.delay.is_empty() {
        warnings.push("missing required `delay` parameters".to_string());
        score -= 30;
        hard_fail = true;
    }
    if expect.require_reverb && tone.reverb.is_empty() {
        warnings.push("missing required `reverb` parameters".to_string());
        score -= 30;
        hard_fail = true;
    }
    if expect.require_eq && tone.eq.is_empty() {
        warnings.push("missing required `eq` parameters".to_string());
        score -= 30;
        hard_fail = true;
    }
    if expect.require_gate {
        let has_gate = tone.effects.iter().any(|e| norm(&e.effect_type).contains("gate"));
        if !has_gate {
            warnings.push("missing required `noise_gate` effect".to_string());
            score -= 30;
            hard_fail = true;
        }
    }

    if expect.forbid_delay && !tone.delay.is_empty() {
        warnings.push("delay present but forbidden by prompt".to_string());
        score -= 25;
        hard_fail = true;
    }
    if expect.forbid_reverb && !tone.reverb.is_empty() {
        warnings.push("reverb present but forbidden by prompt".to_string());
        score -= 25;
        hard_fail = true;
    }
    if expect.forbid_eq && !tone.eq.is_empty() {
        warnings.push("eq present but forbidden by prompt".to_string());
        score -= 25;
        hard_fail = true;
    }
    if expect.forbid_gate {
        let has_gate = tone.effects.iter().any(|e| norm(&e.effect_type).contains("gate"));
        if has_gate {
            warnings.push("noise_gate present but forbidden by prompt".to_string());
            score -= 25;
            hard_fail = true;
        }
    }

    for fx in required_effects {
        let want = norm(fx);
        let has = tone.effects.iter().any(|e| norm(&e.effect_type) == want);
        if !has {
            warnings.push(format!("missing required effect '{}'", fx));
            score -= 20;
            hard_fail = true;
        }
    }

    for fx in forbidden_effects {
        let ban = norm(fx);
        let has = tone.effects.iter().any(|e| norm(&e.effect_type) == ban);
        if has {
            warnings.push(format!("forbidden effect '{}' present", fx));
            score -= 20;
            hard_fail = true;
        }
    }

    // Minimality: flag non-empty sections that weren't requested (soft penalty, not a hard fail).
    if !expect.require_delay && !expect.forbid_delay && !tone.delay.is_empty() {
        warnings.push("delay section present but not requested".to_string());
        score -= 5;
    }
    if !expect.require_reverb && !expect.forbid_reverb && !tone.reverb.is_empty() {
        warnings.push("reverb section present but not requested".to_string());
        score -= 5;
    }
    if !expect.require_eq && !expect.forbid_eq && !tone.eq.is_empty() {
        warnings.push("eq section present but not requested".to_string());
        score -= 5;
    }
    if !expect.require_gate && !expect.forbid_gate && required_effects.is_empty() && !tone.effects.is_empty() {
        warnings.push("effects list present but not requested".to_string());
        score -= 5;
    }

    if expect.require_delay {
        if let Some(mix) = tone.delay.get("mix").copied() {
            if let Some(min) = expect.min_delay_mix {
                if mix + 1e-6 < min {
                    warnings.push(format!("delay mix too low ({:.3} < {:.3})", mix, min));
                    score -= 5;
                }
            }
            if let Some(max) = expect.max_delay_mix {
                if mix - 1e-6 > max {
                    warnings.push(format!("delay mix too high ({:.3} > {:.3})", mix, max));
                    score -= 5;
                }
            }
        } else {
            warnings.push("delay requested but `delay.mix` missing".to_string());
            score -= 3;
        }
    }

    if expect.require_reverb {
        if let Some(mix) = tone.reverb.get("mix").copied() {
            if let Some(min) = expect.min_reverb_mix {
                if mix + 1e-6 < min {
                    warnings.push(format!("reverb mix too low ({:.3} < {:.3})", mix, min));
                    score -= 5;
                }
            }
            if let Some(max) = expect.max_reverb_mix {
                if mix - 1e-6 > max {
                    warnings.push(format!("reverb mix too high ({:.3} > {:.3})", mix, max));
                    score -= 5;
                }
            }
        } else {
            warnings.push("reverb requested but `reverb.mix` missing".to_string());
            score -= 3;
        }
    }

    score = score.clamp(0, 100);
    EngineerEval {
        ok: !hard_fail,
        score,
        warnings,
    }
}

fn evaluate_mapping(
    snapshot: &ReaperSnapshot,
    actions: &[ParameterAction],
    test: &TestCase<'_>,
) -> Vec<String> {
    let mut warnings = Vec::new();

    fn contains_ci(haystack: &str, needle: &str) -> bool {
        haystack.to_lowercase().contains(&needle.to_lowercase())
    }

    let by_index: HashMap<i32, &ReaperPlugin> = snapshot.plugins.iter().map(|p| (p.index, p)).collect();

    let mut touched_plugins: std::collections::HashSet<i32> = std::collections::HashSet::new();
    for a in actions {
        if let ParameterAction::SetParameter { plugin_index, .. } = a {
            touched_plugins.insert(*plugin_index);
        }
    }

    // Forbidden categories: if we touch a plugin matching that category, flag it (even if engineer already failed).
    if test.expect.forbid_delay {
        for pidx in &touched_plugins {
            if let Some(p) = by_index.get(pidx) {
                if contains_ci(&p.name, "delay") {
                    warnings.push(format!("mapping touched delay plugin '{}' despite delay being forbidden", p.name));
                    break;
                }
            }
        }
    }
    if test.expect.forbid_reverb {
        for pidx in &touched_plugins {
            if let Some(p) = by_index.get(pidx) {
                if contains_ci(&p.name, "reverb") || contains_ci(&p.name, "verbate") {
                    warnings.push(format!(
                        "mapping touched reverb plugin '{}' despite reverb being forbidden",
                        p.name
                    ));
                    break;
                }
            }
        }
    }

    // Prefer a specific delay plugin when multiple are present.
    if let Some(prefer) = test.prefer_delay_plugin_contains {
        let preferred: Vec<&ReaperPlugin> = snapshot
            .plugins
            .iter()
            .filter(|p| contains_ci(&p.name, "delay") && contains_ci(&p.name, prefer))
            .collect();
        let any_delay: Vec<&ReaperPlugin> = snapshot
            .plugins
            .iter()
            .filter(|p| contains_ci(&p.name, "delay"))
            .collect();

        if !preferred.is_empty() && any_delay.len() > 1 {
            for pidx in &touched_plugins {
                if let Some(p) = by_index.get(pidx) {
                    if contains_ci(&p.name, "delay") && !contains_ci(&p.name, prefer) {
                        warnings.push(format!(
                            "multiple delay plugins present; expected to prefer '{}' but mapping touched '{}'",
                            prefer, p.name
                        ));
                        break;
                    }
                }
            }
        }
    }

    // Required effects: if plugin exists in chain but mapping never touches it, warn (usually engineer missed it).
    for fx in test.required_effects {
        let mut matching_plugins = snapshot
            .plugins
            .iter()
            .filter(|p| contains_ci(&p.name, fx))
            .peekable();
        if matching_plugins.peek().is_none() {
            continue;
        }
        let any_touched = matching_plugins.any(|p| touched_plugins.contains(&p.index));
        if !any_touched {
            warnings.push(format!(
                "required effect '{}' plugin exists in chain but no SetParameter actions targeted it",
                fx
            ));
        }
    }

    warnings
}

fn prune_for_apply(tone: &ToneParameters, test: &TestCase<'_>) -> (ToneParameters, Vec<String>) {
    fn norm(s: &str) -> String {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }

    let mut out = tone.clone();
    let mut warnings = Vec::new();

    let want_delay = test.expect.require_delay && !test.expect.forbid_delay;
    let want_reverb = test.expect.require_reverb && !test.expect.forbid_reverb;
    let want_eq = test.expect.require_eq && !test.expect.forbid_eq;

    if !want_delay && !out.delay.is_empty() {
        out.delay.clear();
        warnings.push("pruned delay (not requested/forbidden)".to_string());
    }
    if !want_reverb && !out.reverb.is_empty() {
        out.reverb.clear();
        warnings.push("pruned reverb (not requested/forbidden)".to_string());
    }
    if !want_eq && !out.eq.is_empty() {
        out.eq.clear();
        warnings.push("pruned eq (not requested/forbidden)".to_string());
    }

    if want_delay {
        if let Some(mix) = out.delay.get_mut("mix") {
            if let Some(max) = test.expect.max_delay_mix {
                if *mix > max {
                    warnings.push(format!("clamped delay.mix from {:.3} to {:.3}", *mix, max));
                    *mix = max;
                }
            }
            if let Some(min) = test.expect.min_delay_mix {
                if *mix < min {
                    warnings.push(format!("clamped delay.mix from {:.3} to {:.3}", *mix, min));
                    *mix = min;
                }
            }
        }
    }

    if want_reverb {
        if let Some(mix) = out.reverb.get_mut("mix") {
            if let Some(max) = test.expect.max_reverb_mix {
                if *mix > max {
                    warnings.push(format!("clamped reverb.mix from {:.3} to {:.3}", *mix, max));
                    *mix = max;
                }
            }
            if let Some(min) = test.expect.min_reverb_mix {
                if *mix < min {
                    warnings.push(format!("clamped reverb.mix from {:.3} to {:.3}", *mix, min));
                    *mix = min;
                }
            }
        }
    }

    let mut allowed: std::collections::HashSet<String> = std::collections::HashSet::new();
    if test.expect.require_gate && !test.expect.forbid_gate {
        allowed.insert("noise_gate".to_string());
    }
    for fx in test.required_effects {
        allowed.insert(norm(fx));
    }

    if !allowed.is_empty() || !test.forbidden_effects.is_empty() || test.expect.forbid_gate {
        let before = out.effects.len();
        out.effects.retain(|e| {
            let et = norm(&e.effect_type);
            if test.expect.forbid_gate && et.contains("gate") {
                return false;
            }
            if test
                .forbidden_effects
                .iter()
                .any(|ban| et == norm(ban))
            {
                return false;
            }
            if allowed.is_empty() {
                return false;
            }
            allowed.contains(&et)
        });
        if out.effects.len() != before {
            warnings.push(format!("pruned effects: {} -> {}", before, out.effects.len()));
        }
    }

    (out, warnings)
}

fn offline_engineer(name: &str, _prompt: &str) -> EngineerOut {
    // Deterministic adversarial-ish outputs to stress the chain without calling a model.
    let mut amp = HashMap::new();
    let mut eq = HashMap::new();
    let mut reverb = HashMap::new();
    let mut delay = HashMap::new();
    let mut effects = Vec::new();

    match name {
        "Delay bypassed section" => {
            delay.insert("time".to_string(), 0.95); // will be clamped by mock for ReaDelay time to 0.9
            delay.insert("mix".to_string(), 0.15);
            delay.insert("feedback".to_string(), 0.25);
        }
        "Dual delay prefer ReaDelay" => {
            delay.insert("time".to_string(), 0.12);
            delay.insert("mix".to_string(), 0.12);
            delay.insert("feedback".to_string(), 0.18);
        }
        "Shoegaze wall (niche)" => {
            delay.insert("time".to_string(), 0.55);
            delay.insert("mix".to_string(), 0.25);
            delay.insert("feedback".to_string(), 0.35);
            reverb.insert("mix".to_string(), 0.35);
            reverb.insert("room_size".to_string(), 0.8);
            effects.push(EffectParameters {
                effect_type: "chorus".to_string(),
                parameters: HashMap::from([
                    ("mix".to_string(), 0.35),
                    ("depth".to_string(), 0.6),
                    ("rate".to_string(), 0.35),
                ]),
            });
        }
        "Reverb bypassed section" => {
            reverb.insert("mix".to_string(), 0.08);
            reverb.insert("room_size".to_string(), 0.2);
        }
        "Gate plugin disabled" => {
            effects.push(EffectParameters {
                effect_type: "noise_gate".to_string(),
                parameters: HashMap::from([
                    ("threshold".to_string(), 0.2),
                    ("attack".to_string(), 0.1),
                    ("release".to_string(), 0.4),
                ]),
            });
            amp.insert("gain".to_string(), 0.85);
        }
        "Gate section disabled" => {
            effects.push(EffectParameters {
                effect_type: "noise_gate".to_string(),
                parameters: HashMap::from([
                    ("threshold".to_string(), 0.25),
                    ("attack".to_string(), 0.1),
                    ("release".to_string(), 0.5),
                ]),
            });
        }
        "Reverb missing (should load)" => {
            reverb.insert("mix".to_string(), 0.1);
            reverb.insert("room_size".to_string(), 0.2);
        }
        "EQ bypassed section" => {
            eq.insert("250Hz".to_string(), -9.0);
            amp.insert("presence".to_string(), 0.65);
        }
        "Swedish chainsaw (niche)" => {
            effects.push(EffectParameters {
                effect_type: "distortion".to_string(),
                parameters: HashMap::from([
                    ("drive".to_string(), 0.95),
                    ("low".to_string(), 0.9),
                    ("high".to_string(), 0.9),
                ]),
            });
            eq.insert("250Hz".to_string(), -8.0);
            eq.insert("1kHz".to_string(), -3.0);
        }
        "Funk compressor (niche)" => {
            effects.push(EffectParameters {
                effect_type: "compressor".to_string(),
                parameters: HashMap::from([
                    ("threshold".to_string(), 0.35),
                    ("ratio".to_string(), 0.35),
                    ("attack".to_string(), 0.35),
                    ("release".to_string(), 0.45),
                    ("mix".to_string(), 1.0),
                ]),
            });
            amp.insert("gain".to_string(), 0.25);
        }
        "Tubescreamer tighten (niche)" => {
            effects.push(EffectParameters {
                effect_type: "overdrive".to_string(),
                parameters: HashMap::from([
                    ("drive".to_string(), 0.15),
                    ("level".to_string(), 0.85),
                    ("tone".to_string(), 0.6),
                ]),
            });
        }
        "Contradiction: keep reverb OFF" => {
            amp.insert("gain".to_string(), 0.55);
            amp.insert("presence".to_string(), 0.6);
        }
        _ => {
            amp.insert("gain".to_string(), 1.2);
            amp.insert("presence".to_string(), -0.2);
            delay.insert("time".to_string(), 1.5);
            effects.push(EffectParameters {
                effect_type: "noise_gate".to_string(),
                parameters: HashMap::from([("threshold".to_string(), 2.0)]),
            });
            eq.insert("250Hz".to_string(), -20.0);
        }
    }

    EngineerOut {
        description: format!("OFFLINE engineer for '{}'", name),
        parameters: ToneParameters {
            amp,
            eq,
            effects,
            reverb,
            delay,
        },
    }
}

fn read_api_key(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(Path::new(path)).context("read api.txt")?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // Accept KEY=VALUE styles.
        if let Some((k, v)) = trimmed.split_once('=') {
            let k = k.trim();
            let v = v.trim();
            if !v.is_empty()
                && matches!(
                    k.to_ascii_uppercase().as_str(),
                    "API_KEY" | "GEMINI_API_KEY" | "VERTEX_API_KEY"
                )
            {
                return Ok(v.to_string());
            }
        }
        // Otherwise, take the first whitespace token.
        if let Some(tok) = trimmed.split_whitespace().next() {
            if !tok.is_empty() {
                return Ok(tok.to_string());
            }
        }
    }
    Err(anyhow!("api.txt had no key token"))
}

fn resolve_api_key() -> Result<String> {
    let mut args = std::env::args().skip(1).peekable();
    let mut api_key_arg: Option<String> = None;
    let mut api_key_path: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--api-key" => api_key_arg = args.next(),
            "--api-key-path" => api_key_path = args.next(),
            _ => {}
        }
    }

    if let Some(key) = api_key_arg {
        let k = key.trim().to_string();
        if !k.is_empty() {
            return Ok(k);
        }
    }

    for env_name in ["API_KEY", "GEMINI_API_KEY", "VERTEX_API_KEY"] {
        if let Ok(v) = std::env::var(env_name) {
            let t = v.trim();
            if !t.is_empty() {
                return Ok(t.to_string());
            }
        }
    }

    if let Some(path) = api_key_path {
        return read_api_key(&path);
    }

    for candidate in ["api.txt", "../api.txt", "../../api.txt", "../../../api.txt"] {
        if Path::new(candidate).exists() {
            return read_api_key(candidate);
        }
    }

    Err(anyhow!(
        "No API key found; pass --api-key, set env API_KEY, or create ../../api.txt"
    ))
}

fn start_mock_server() -> Result<Child> {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let child = Command::new(python)
        .arg("scripts/mock_reaper.py")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn mock server")?;
    Ok(child)
}

async fn wait_for_ping() -> Result<()> {
    let client = Client::new();
    for _ in 0..50 {
        if let Ok(resp) = client.get(format!("{}/ping", BASE_URL)).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(anyhow!("mock server did not respond to /ping"))
}

async fn reset_scenario(client: &Client, scenario: &str) -> Result<()> {
    let url = format!("{}/__reset?scenario={}", BASE_URL, scenario);
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("reset failed: {}", resp.status()));
    }
    Ok(())
}

async fn collect_snapshot(client: &Client, track: i32) -> Result<ReaperSnapshot> {
    let tracks: Value = client
        .get(format!("{}/tracks", BASE_URL))
        .send()
        .await?
        .json()
        .await?;

    let tracks_arr = tracks["tracks"].as_array().ok_or_else(|| anyhow!("bad /tracks"))?;
    let track_obj = tracks_arr
        .iter()
        .find(|t| t["index"].as_i64() == Some(track as i64))
        .ok_or_else(|| anyhow!("track not found"))?;

    let track_name = track_obj["name"].as_str().unwrap_or("Track").to_string();
    let fx_list = track_obj["fx_list"].as_array().cloned().unwrap_or_default();

    let mut plugins = Vec::new();
    for fx in fx_list {
        let fx_index = fx["index"].as_i64().unwrap_or(0) as i32;
        let fx_name = fx["name"].as_str().unwrap_or("").to_string();
        let enabled = fx["enabled"].as_bool().unwrap_or(true);

        let params_json: Value = client
            .get(format!("{}/fx/params", BASE_URL))
            .query(&[("track", track), ("fx", fx_index)])
            .send()
            .await?
            .json()
            .await?;

        let params_arr = params_json["params"].as_array().cloned().unwrap_or_default();
        let mut parameters = Vec::new();
        for p in params_arr {
            parameters.push(ReaperParameter {
                index: p["index"].as_i64().unwrap_or(0) as i32,
                name: p["name"].as_str().unwrap_or("").to_string(),
                current_value: p["value"].as_f64().unwrap_or(0.0),
                display_value: p["display"].as_str().unwrap_or("").to_string(),
                unit: p["unit"].as_str().unwrap_or("").to_string(),
                format_hint: p["format_hint"].as_str().unwrap_or("raw").to_string(),
            });
        }

        plugins.push(ReaperPlugin {
            index: fx_index,
            name: fx_name,
            enabled,
            parameters,
        });
    }

    Ok(ReaperSnapshot {
        track_index: track,
        track_name,
        plugins,
    })
}

async fn apply_actions(client: &Client, actions: &[ParameterAction]) -> Result<ApplyRes> {
    let mut logs = Vec::new();
    let mut warnings = Vec::new();

    for a in actions {
        match a {
            ParameterAction::LoadPlugin { track, plugin_name, .. } => {
                let resp: Value = client
                    .post(format!("{}/fx/add", BASE_URL))
                    .json(&json!({"track": track, "plugin": plugin_name}))
                    .send()
                    .await?
                    .json()
                    .await?;
                let slot = resp["fx_index"].as_i64().unwrap_or(-1);
                logs.push(format!("loaded '{}' slot {}", plugin_name, slot));
            }
            ParameterAction::EnablePlugin { track, plugin_index, .. } => {
                let resp: Value = client
                    .post(format!("{}/fx/toggle", BASE_URL))
                    .json(&json!({"track": track, "fx": plugin_index, "enabled": true}))
                    .send()
                    .await?
                    .json()
                    .await?;
                if resp["enabled"].as_bool() != Some(true) {
                    warnings.push(format!("enable failed for fx {}", plugin_index));
                }
                logs.push(format!("enabled fx {}", plugin_index));
            }
            ParameterAction::SetParameter {
                track,
                plugin_index,
                param_index,
                param_name,
                value,
                ..
            } => {
                let resp: Value = client
                    .post(format!("{}/fx/param_index", BASE_URL))
                    .json(&json!({
                        "track": track,
                        "fx": plugin_index,
                        "param_index": param_index,
                        "value": value
                    }))
                    .send()
                    .await?
                    .json()
                    .await?;
                let applied = resp["value"].as_f64().unwrap_or(*value);
                if (applied - *value).abs() > 0.02 {
                    warnings.push(format!(
                        "apply mismatch {}[{}] expected {:.3} got {:.3}",
                        param_name, param_index, value, applied
                    ));
                }
                logs.push(format!("set fx {} param {} -> {:.3}", plugin_index, param_name, applied));
            }
        }
    }

    Ok(ApplyRes { logs, warnings })
}

fn check_invariants(before: &ReaperSnapshot, after: &ReaperSnapshot, actions: &[ParameterAction]) -> Invariants {
    let mut enable_action_before_set = true;
    let mut first_enable: HashMap<i32, usize> = HashMap::new();
    let mut first_set: HashMap<i32, usize> = HashMap::new();

    for (idx, a) in actions.iter().enumerate() {
        match a {
            ParameterAction::EnablePlugin { plugin_index, .. } => {
                first_enable.entry(*plugin_index).or_insert(idx);
            }
            ParameterAction::SetParameter { plugin_index, .. } => {
                first_set.entry(*plugin_index).or_insert(idx);
            }
            _ => {}
        }
    }

    for p in &before.plugins {
        if !p.enabled {
            let Some(set_idx) = first_set.get(&p.index) else { continue };
            let Some(enable_idx) = first_enable.get(&p.index) else {
                enable_action_before_set = false;
                continue;
            };
            if enable_idx > set_idx {
                enable_action_before_set = false;
            }
        }
    }

    let mut plugins_enabled_if_params_set = true;
    let mut no_param_changes_while_inactive = true;

    let mut plugins_with_set: std::collections::HashSet<i32> = std::collections::HashSet::new();
    for a in actions {
        if let ParameterAction::SetParameter { plugin_index, .. } = a {
            plugins_with_set.insert(*plugin_index);
        }
    }

    for pidx in &plugins_with_set {
        if let Some(p) = after.plugins.iter().find(|p| p.index == *pidx) {
            if !p.enabled {
                plugins_enabled_if_params_set = false;
            }
        }
    }

    let before_map: HashMap<i32, &ReaperPlugin> = before.plugins.iter().map(|p| (p.index, p)).collect();

    fn is_bypass(name: &str) -> bool {
        name.to_lowercase().contains("bypass")
    }

    fn is_enable(name: &str) -> bool {
        let n = name.to_lowercase();
        n.contains("enable") || n.contains("enabled") || n.contains("active") || n.ends_with(" on")
    }

    fn is_gate_param(name: &str) -> bool {
        is_bypass(name) || is_enable(name)
    }

    for p in &after.plugins {
        let Some(p_before) = before_map.get(&p.index) else { continue };
        let mut before_params: HashMap<i32, f64> = HashMap::new();
        for bp in &p_before.parameters {
            before_params.insert(bp.index, bp.current_value);
        }

        // Conservative: if any gate indicates inactive, require no other param changes.
        let mut inactive = false;
        for ap in &p.parameters {
            if is_bypass(&ap.name) && ap.current_value >= 0.5 {
                inactive = true;
            }
            if is_enable(&ap.name) && ap.current_value < 0.5 {
                inactive = true;
            }
        }

        if !inactive {
            continue;
        }

        for ap in &p.parameters {
            if is_gate_param(&ap.name) {
                continue;
            }
            let before_v = before_params.get(&ap.index).copied().unwrap_or(ap.current_value);
            if (ap.current_value - before_v).abs() > 1e-6 {
                no_param_changes_while_inactive = false;
                break;
            }
        }
    }

    // Delay bypass cleared if Delay Time/Feedback/Mix set
    let mut delay_set = false;
    let mut delay_bypass_cleared = true;

    // Gate enable cleared if Threshold set (for mock "Gate Enable" param)
    let mut threshold_set = false;
    let mut gate_enable_ok = true;

    // Reverb bypass cleared if Mix/Room Size set
    let mut reverb_set = false;
    let mut reverb_bypass_cleared = true;

    // EQ bypass cleared if band gain/freq set
    let mut eq_set = false;
    let mut eq_bypass_cleared = true;

    for p in &after.plugins {
        let pnorm = p.name.to_lowercase();
        if pnorm.contains("delay") {
            let bypass = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("bypass"))
                .map(|x| x.current_value);
            let time = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("time"))
                .map(|x| x.current_value);
            if let (Some(b), Some(t)) = (bypass, time) {
                // If time changed from default-ish, consider "delay set"
                if (t - 0.3).abs() > 0.0001 {
                    delay_set = true;
                    if b >= 0.5 {
                        delay_bypass_cleared = false;
                    }
                }
            }
        }

        if pnorm.contains("gate") {
            let enable = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("enable"))
                .map(|x| x.current_value);
            let threshold = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase() == "threshold")
                .map(|x| x.current_value);
            if let Some(th) = threshold {
                if (th - 0.5).abs() > 0.0001 {
                    threshold_set = true;
                    if enable.unwrap_or(1.0) < 0.5 {
                        gate_enable_ok = false;
                    }
                }
            }
        }

        if pnorm.contains("reverb") || pnorm.contains("verbate") {
            let bypass = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("bypass"))
                .map(|x| x.current_value);
            let mix = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase() == "mix")
                .map(|x| x.current_value);
            let room = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("room"))
                .map(|x| x.current_value);
            if let (Some(b), Some(m)) = (bypass, mix) {
                if (m - 0.1).abs() > 0.0001 || room.map(|r| (r - 0.25).abs() > 0.0001).unwrap_or(false) {
                    reverb_set = true;
                    if b >= 0.5 {
                        reverb_bypass_cleared = false;
                    }
                }
            }
        }

        if pnorm.contains("reaeq") || pnorm.contains(" eq") || pnorm.contains("equal") {
            let bypass = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("bypass"))
                .map(|x| x.current_value);
            let gain = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("gain"))
                .map(|x| x.current_value);
            let freq = p
                .parameters
                .iter()
                .find(|x| x.name.to_lowercase().contains("freq"))
                .map(|x| x.current_value);
            if gain.map(|g| (g - 0.5).abs() > 0.0001).unwrap_or(false)
                || freq.map(|f| (f - 0.4).abs() > 0.0001).unwrap_or(false)
            {
                eq_set = true;
                if bypass.unwrap_or(0.0) >= 0.5 {
                    eq_bypass_cleared = false;
                }
            }
        }
    }

    Invariants {
        enable_action_before_set,
        delay_bypass_cleared_if_delay_set: !delay_set || delay_bypass_cleared,
        gate_enable_cleared_if_threshold_set: !threshold_set || gate_enable_ok,
        reverb_bypass_cleared_if_reverb_set: !reverb_set || reverb_bypass_cleared,
        eq_bypass_cleared_if_eq_set: !eq_set || eq_bypass_cleared,
        plugins_enabled_if_params_set,
        no_param_changes_while_inactive,
    }
}

async fn gemini_tone_engineer(
    client: &Client,
    credential: &str,
    model: &str,
    prompt: &str,
) -> Result<EngineerOut> {
    let req = json!({
        "systemInstruction": { "parts": [ { "text": TONE_ENGINEER_SYSTEM_PROMPT } ] },
        "contents": [ { "role": "user", "parts": [ { "text": prompt } ] } ],
        "generationConfig": {
            "temperature": 0.2,
            "topP": 0.95,
            "maxOutputTokens": 4096,
            "responseMimeType": "application/json"
        }
    });

    // Vertex AI public publisher endpoint with API key (matches /var/www/wp-panel implementation).
    let resp = vertex_generate_with_key(client, credential, model, &req).await?;
    parse_engineer_out(resp, prompt)
}

async fn gemini_tone_engineer_repair(
    client: &Client,
    credential: &str,
    model: &str,
    prompt: &str,
    prior_description: &str,
    prior_params: &ToneParameters,
    issues: &[String],
) -> Result<EngineerOut> {
    let prior_json = json!({
        "description": prior_description,
        "parameters": tone_parameters_to_value(prior_params),
    });

    let issue_text = issues
        .iter()
        .take(8)
        .map(|s| format!("- {}", s))
        .collect::<Vec<_>>()
        .join("\n");

    let repair_prompt = format!(
        "Your previous JSON output was missing required content or sections.\n\nIssues:\n{}\n\nPrevious JSON:\n{}\n\nReturn a corrected JSON object ONLY, matching the schema exactly.",
        issue_text,
        serde_json::to_string_pretty(&prior_json).unwrap_or_else(|_| prior_json.to_string())
    );

    let req = json!({
        "systemInstruction": { "parts": [ { "text": TONE_ENGINEER_SYSTEM_PROMPT } ] },
        "contents": [
            { "role": "user", "parts": [ { "text": prompt } ] },
            { "role": "user", "parts": [ { "text": repair_prompt } ] }
        ],
        "generationConfig": {
            "temperature": 0.1,
            "topP": 0.95,
            "maxOutputTokens": 4096,
            "responseMimeType": "application/json"
        }
    });

    let resp = vertex_generate_with_key(client, credential, model, &req).await?;
    parse_engineer_out(resp, prompt)
}

fn tone_parameters_to_value(p: &ToneParameters) -> Value {
    let effects: Vec<Value> = p
        .effects
        .iter()
        .map(|e| json!({"effect_type": e.effect_type, "parameters": e.parameters}))
        .collect();
    json!({
        "amp": p.amp,
        "eq": p.eq,
        "effects": effects,
        "reverb": p.reverb,
        "delay": p.delay,
    })
}

fn parse_engineer_out(resp: Value, _prompt: &str) -> Result<EngineerOut> {
    if let Some(err) = resp.get("error") {
        return Err(anyhow!("Gemini error: {}", summarize(&err.to_string(), 400)));
    }

    // Join all text parts, if present.
    let parts = resp["candidates"][0]["content"]["parts"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut text = String::new();
    for p in parts {
        if let Some(t) = p.get("text").and_then(|v| v.as_str()) {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(t);
        }
    }

    if text.trim().is_empty() {
        return Err(anyhow!(
            "no text in gemini response; keys: {}",
            resp.as_object()
                .map(|o| o.keys().cloned().collect::<Vec<_>>().join(","))
                .unwrap_or_else(|| "<non-object>".to_string())
        ));
    }

    let json_text = extract_json(&text).ok_or_else(|| anyhow!("no JSON object found in model output"))?;
    let v: Value = serde_json::from_str(&json_text).context("parse model JSON")?;

    let desc = v["description"].as_str().unwrap_or("").to_string();
    let params = parse_tone_parameters(&v["parameters"]).context("parse parameters")?;
    Ok(EngineerOut {
        description: desc,
        parameters: params,
    })
}

async fn vertex_generate_with_key(client: &Client, api_key: &str, model: &str, req: &Value) -> Result<Value> {
    let url = format!(
        "https://aiplatform.googleapis.com/v1/publishers/google/models/{}:generateContent",
        model
    );
    let http = client.post(url).query(&[("key", api_key)]).json(req).send().await?;
    let status = http.status();
    let body_text = http.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("Vertex HTTP {}: {}", status, summarize(&body_text, 300)));
    }
    let v: Value = serde_json::from_str(&body_text)?;
    Ok(v)
}

fn extract_json(text: &str) -> Option<String> {
    // Handle ```json blocks or raw JSON.
    if let Some(start) = text.find("```json") {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            return Some(after[..end].trim().to_string());
        }
    }
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        if let Some(end) = after.find("```") {
            let candidate = after[..end].trim();
            if candidate.starts_with('{') {
                return Some(candidate.to_string());
            }
        }
    }
    if let (Some(s), Some(e)) = (text.find('{'), text.rfind('}')) {
        if e > s {
            return Some(text[s..=e].to_string());
        }
    }
    None
}

fn parse_tone_parameters(v: &Value) -> Result<ToneParameters> {
    Ok(ToneParameters {
        amp: parse_map_f64(&v["amp"])?,
        eq: parse_map_f64(&v["eq"])?,
        effects: parse_effects(&v["effects"])?,
        reverb: parse_map_f64(&v["reverb"])?,
        delay: parse_map_f64(&v["delay"])?,
    })
}

fn parse_map_f64(v: &Value) -> Result<HashMap<String, f64>> {
    let mut out = HashMap::new();
    let Some(obj) = v.as_object() else {
        return Ok(out);
    };
    for (k, vv) in obj {
        if let Some(n) = vv.as_f64() {
            out.insert(k.to_string(), n);
        }
    }
    Ok(out)
}

fn parse_effects(v: &Value) -> Result<Vec<EffectParameters>> {
    let mut out = Vec::new();
    let Some(arr) = v.as_array() else {
        return Ok(out);
    };
    for e in arr {
        let et = e["effect_type"].as_str().unwrap_or("").to_string();
        let params = parse_map_f64(&e["parameters"])?;
        if !et.is_empty() {
            out.push(EffectParameters {
                effect_type: et,
                parameters: params,
            });
        }
    }
    Ok(out)
}
