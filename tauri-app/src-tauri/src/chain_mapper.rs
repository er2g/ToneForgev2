//! Top-Level Chain Mapper
//!
//! Goal: deterministically map ToneParameters -> REAPER ParameterAction list,
//! keeping AI away from large parameter spaces and unit conversions.

use crate::parameter_ai::{ParameterAction, ReaperPlugin, ReaperSnapshot};
use crate::tone_encyclopedia::{EffectParameters, ToneParameters};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ChainMapperConfig {
    pub allow_load_plugins: bool,
    pub max_eq_points: usize,
}

impl Default for ChainMapperConfig {
    fn default() -> Self {
        Self {
            allow_load_plugins: true,
            max_eq_points: 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChainMappingResult {
    pub actions: Vec<ParameterAction>,
    pub summary: String,
    pub warnings: Vec<String>,
    pub requires_resnapshot: bool,
}

pub struct ChainMapper {
    config: ChainMapperConfig,
}

impl ChainMapper {
    pub fn new(config: ChainMapperConfig) -> Self {
        Self { config }
    }

    pub fn map(&self, tone_params: &ToneParameters, snapshot: &ReaperSnapshot) -> ChainMappingResult {
        let track = snapshot.track_index;
        let mut actions: Vec<ParameterAction> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut requires_resnapshot = false;

        // Amp
        let amp_plugin = pick_best_plugin(snapshot, &role_keywords_amp());
        if let Some(plugin) = amp_plugin {
            if !plugin.enabled && !tone_params.amp.is_empty() {
                actions.push(ParameterAction::EnablePlugin {
                    track,
                    plugin_index: plugin.index,
                    plugin_name: plugin.name.clone(),
                    reason: "Enable amp plugin for tone mapping".to_string(),
                });
            }
            map_param_group(
                track,
                plugin,
                &tone_params.amp,
                "amp",
                &mut actions,
                &mut warnings,
            );
        } else if !tone_params.amp.is_empty() {
            warnings.push("No suitable amp plugin found; amp parameters were not applied".to_string());
        }

        // Effects (gate/overdrive/etc.)
        for effect in &tone_params.effects {
            let role = normalize_token(&effect.effect_type);
            let keywords = role_keywords_for_effect(&role);
            let plugin = pick_best_plugin(snapshot, &keywords);
            if let Some(plugin) = plugin {
                if !plugin.enabled && !effect.parameters.is_empty() {
                    actions.push(ParameterAction::EnablePlugin {
                        track,
                        plugin_index: plugin.index,
                        plugin_name: plugin.name.clone(),
                        reason: format!("Enable '{}' plugin for tone mapping", effect.effect_type),
                    });
                }
                map_effect_group(track, plugin, effect, &mut actions, &mut warnings);
            } else if self.config.allow_load_plugins {
                if let Some(default_fx) = default_plugin_for_effect(&role) {
                    actions.push(ParameterAction::LoadPlugin {
                        track,
                        plugin_name: default_fx.to_string(),
                        position: None,
                        reason: format!("Load missing effect plugin for '{}'", effect.effect_type),
                    });
                    requires_resnapshot = true;
                } else {
                    warnings.push(format!(
                        "No suitable plugin found for effect '{}'; skipped",
                        effect.effect_type
                    ));
                }
            } else {
                warnings.push(format!(
                    "No suitable plugin found for effect '{}'; skipped",
                    effect.effect_type
                ));
            }
        }

        // Reverb
        if !tone_params.reverb.is_empty() {
            let plugin = pick_best_plugin(snapshot, &role_keywords_reverb());
            if let Some(plugin) = plugin {
                if !plugin.enabled {
                    actions.push(ParameterAction::EnablePlugin {
                        track,
                        plugin_index: plugin.index,
                        plugin_name: plugin.name.clone(),
                        reason: "Enable reverb plugin for tone mapping".to_string(),
                    });
                }
                map_param_group(
                    track,
                    plugin,
                    &tone_params.reverb,
                    "reverb",
                    &mut actions,
                    &mut warnings,
                );
            } else if self.config.allow_load_plugins {
                actions.push(ParameterAction::LoadPlugin {
                    track,
                    plugin_name: "ReaVerbate (Cockos)".to_string(),
                    position: None,
                    reason: "Load missing reverb plugin".to_string(),
                });
                requires_resnapshot = true;
            } else {
                warnings.push("No suitable reverb plugin found; skipped".to_string());
            }
        }

        // Delay
        if !tone_params.delay.is_empty() {
            let plugin = pick_best_plugin(snapshot, &role_keywords_delay());
            if let Some(plugin) = plugin {
                if !plugin.enabled {
                    actions.push(ParameterAction::EnablePlugin {
                        track,
                        plugin_index: plugin.index,
                        plugin_name: plugin.name.clone(),
                        reason: "Enable delay plugin for tone mapping".to_string(),
                    });
                }
                map_param_group(
                    track,
                    plugin,
                    &tone_params.delay,
                    "delay",
                    &mut actions,
                    &mut warnings,
                );
            } else if self.config.allow_load_plugins {
                actions.push(ParameterAction::LoadPlugin {
                    track,
                    plugin_name: "ReaDelay (Cockos)".to_string(),
                    position: None,
                    reason: "Load missing delay plugin".to_string(),
                });
                requires_resnapshot = true;
            } else {
                warnings.push("No suitable delay plugin found; skipped".to_string());
            }
        }

        // EQ (initial: ReaEQ-focused)
        if !tone_params.eq.is_empty() {
            let plugin = pick_best_plugin(snapshot, &role_keywords_eq());
            if let Some(plugin) = plugin {
                if !plugin.enabled {
                    actions.push(ParameterAction::EnablePlugin {
                        track,
                        plugin_index: plugin.index,
                        plugin_name: plugin.name.clone(),
                        reason: "Enable EQ plugin for tone mapping".to_string(),
                    });
                }
                if contains_token(&plugin.name, "reaeq") {
                    map_eq_reaeq(track, plugin, &tone_params.eq, self.config.max_eq_points, &mut actions, &mut warnings);
                } else {
                    warnings.push(format!(
                        "EQ plugin '{}' is not supported by deterministic mapper yet; EQ skipped",
                        plugin.name
                    ));
                }
            } else if self.config.allow_load_plugins {
                actions.push(ParameterAction::LoadPlugin {
                    track,
                    plugin_name: "ReaEQ (Cockos)".to_string(),
                    position: None,
                    reason: "Load missing EQ plugin".to_string(),
                });
                requires_resnapshot = true;
            } else {
                warnings.push("No suitable EQ plugin found; skipped".to_string());
            }
        }

        let actions = ensure_prerequisites(actions, snapshot, &mut warnings);
        let actions = plan_actions(actions, &mut warnings);
        let summary = build_summary(&actions, requires_resnapshot);

        ChainMappingResult {
            actions,
            summary,
            warnings,
            requires_resnapshot,
        }
    }
}

fn ensure_prerequisites(
    mut actions: Vec<ParameterAction>,
    snapshot: &ReaperSnapshot,
    warnings: &mut Vec<String>,
) -> Vec<ParameterAction> {
    // Ensure plugin is enabled before any SetParameter (guard against bugs/missed enables).
    let plugin_enabled: HashMap<i32, bool> = snapshot.plugins.iter().map(|p| (p.index, p.enabled)).collect();
    let mut has_enable: HashSet<i32> = HashSet::new();
    let mut needs_enable: HashSet<i32> = HashSet::new();

    for a in &actions {
        if let ParameterAction::EnablePlugin { plugin_index, .. } = a {
            has_enable.insert(*plugin_index);
        }
    }

    for a in &actions {
        if let ParameterAction::SetParameter { plugin_index, .. } = a {
            if plugin_enabled.get(plugin_index).copied() == Some(false) && !has_enable.contains(plugin_index) {
                needs_enable.insert(*plugin_index);
            }
        }
    }

    for plugin_index in needs_enable {
        if let Some(plugin) = snapshot.plugins.iter().find(|p| p.index == plugin_index) {
            warnings.push(format!(
                "Plugin '{}' was disabled but has SetParameter actions; inserting EnablePlugin",
                plugin.name
            ));
            actions.push(ParameterAction::EnablePlugin {
                track: snapshot.track_index,
                plugin_index,
                plugin_name: plugin.name.clone(),
                reason: "Auto-enable plugin because parameters will be set".to_string(),
            });
        }
    }

    // Heuristic: enable module/section if a matching bypass/enable parameter exists and is currently inactive.
    // This helps avoid “changing params in an inactive section”.
    let mut inserted_section_toggles: HashSet<(i32, i32)> = HashSet::new(); // (plugin_index, gate_param_index)

    // Build per-plugin gate parameter candidates
    let mut plugin_gates: HashMap<i32, Vec<GateParam>> = HashMap::new();
    for plugin in &snapshot.plugins {
        let mut gates = Vec::new();
        for p in &plugin.parameters {
            if let Some(kind) = gate_kind(&p.name) {
                gates.push(GateParam {
                    param_index: p.index,
                    param_name: p.name.clone(),
                    current_value: p.current_value,
                    kind,
                    module_tokens: module_tokens(&p.name),
                });
            }
        }
        if !gates.is_empty() {
            plugin_gates.insert(plugin.index, gates);
        }
    }

    // For each SetParameter, ensure related gate is enabled if clearly matchable.
    let mut extra_actions = Vec::new();
    for a in &actions {
        let ParameterAction::SetParameter {
            track,
            plugin_index,
            param_index,
            param_name,
            ..
        } = a
        else {
            continue;
        };

        let gates = plugin_gates.get(plugin_index);
        let Some(gates) = gates else {
            continue;
        };

        // Skip if the param being set is itself a gate.
        if gates.iter().any(|g| g.param_index == *param_index) {
            continue;
        }

        let target_tokens = module_tokens(param_name);
        // Pick best matching gate by token overlap (module-level), requiring overlap >= 1.
        // Fallbacks:
        // - If no module token match but there is exactly one gate parameter, treat it as a global gate.
        // - If the gate parameter itself has no module tokens (e.g. "Bypass"), treat it as global.
        let gate: Option<&GateParam> = if !target_tokens.is_empty() {
            let mut best: Option<(&GateParam, usize)> = None;
            for g in gates {
                let overlap = g.module_tokens.intersection(&target_tokens).count();
                if overlap == 0 {
                    continue;
                }
                match best {
                    None => best = Some((g, overlap)),
                    Some((_, best_overlap)) if overlap > best_overlap => best = Some((g, overlap)),
                    _ => {}
                }
            }
            best.map(|(g, _)| g)
        } else {
            None
        };

        let gate = match gate {
            Some(g) => Some(g),
            None if gates.len() == 1 => gates.first(),
            None => gates.iter().find(|g| g.module_tokens.is_empty()),
        };

        let Some(gate) = gate else { continue };

        if inserted_section_toggles.contains(&(*plugin_index, gate.param_index)) {
            continue;
        }

        if gate_is_inactive(gate) {
            inserted_section_toggles.insert((*plugin_index, gate.param_index));
            warnings.push(format!(
                "Section gate '{}' appears inactive; inserting toggle before setting '{}'",
                gate.param_name, param_name
            ));
            extra_actions.push(ParameterAction::SetParameter {
                track: *track,
                plugin_index: *plugin_index,
                param_index: gate.param_index,
                param_name: gate.param_name.clone(),
                value: gate_enable_value(gate),
                reason: format!("Auto-enable section for '{}'", param_name),
            });
        }
    }

    actions.extend(extra_actions);
    actions
}

#[derive(Debug, Clone, Copy)]
enum GateKind {
    Bypass,
    Enable,
}

#[derive(Debug, Clone)]
struct GateParam {
    param_index: i32,
    param_name: String,
    current_value: f64,
    kind: GateKind,
    module_tokens: HashSet<String>,
}

fn gate_kind(name: &str) -> Option<GateKind> {
    let n = normalize_token(name);
    if n.contains("bypass") {
        return Some(GateKind::Bypass);
    }
    if n.contains("enable") || n.contains("enabled") || n.ends_with("on") || n.contains("active") {
        return Some(GateKind::Enable);
    }
    None
}

fn gate_is_inactive(gate: &GateParam) -> bool {
    // Normalized params: treat >= 0.5 as "true".
    match gate.kind {
        GateKind::Bypass => gate.current_value >= 0.5, // bypassed
        GateKind::Enable => gate.current_value < 0.5,  // disabled
    }
}

fn gate_enable_value(gate: &GateParam) -> f64 {
    match gate.kind {
        GateKind::Bypass => 0.0, // disable bypass
        GateKind::Enable => 1.0, // enable
    }
}

fn module_tokens(name: &str) -> HashSet<String> {
    let raw_tokens: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect();

    let stop: HashSet<&'static str> = [
        "enable",
        "enabled",
        "bypass",
        "on",
        "off",
        "active",
        "switch",
        "button",
        "band",
        "freq",
        "frequency",
        "gain",
        "level",
        "mix",
        "amount",
    ]
    .into_iter()
    .collect();

    raw_tokens
        .into_iter()
        .filter(|t| t.chars().any(|c| c.is_alphabetic()))
        .filter(|t| !stop.contains(t.as_str()))
        .collect()
}

fn build_summary(actions: &[ParameterAction], requires_resnapshot: bool) -> String {
    let mut set_count = 0usize;
    let mut enable_count = 0usize;
    let mut load_count = 0usize;

    for a in actions {
        match a {
            ParameterAction::SetParameter { .. } => set_count += 1,
            ParameterAction::EnablePlugin { .. } => enable_count += 1,
            ParameterAction::LoadPlugin { .. } => load_count += 1,
        }
    }

    let mut parts = Vec::new();
    if load_count > 0 {
        parts.push(format!("load {} plugin(s)", load_count));
    }
    if enable_count > 0 {
        parts.push(format!("enable {} plugin(s)", enable_count));
    }
    if set_count > 0 {
        parts.push(format!("set {} parameter(s)", set_count));
    }
    if parts.is_empty() {
        parts.push("no actions".to_string());
    }
    if requires_resnapshot {
        parts.push("requires resnapshot".to_string());
    }
    parts.join(", ")
}

fn plan_actions(mut actions: Vec<ParameterAction>, warnings: &mut Vec<String>) -> Vec<ParameterAction> {
    // Clamp SetParameter values and normalize NaN/Inf
    for a in &mut actions {
        if let ParameterAction::SetParameter { value, .. } = a {
            if !value.is_finite() {
                warnings.push("Non-finite parameter value encountered; clamping to 0.5".to_string());
                *value = 0.5;
            }
            if *value < 0.0 {
                warnings.push(format!("Value {} < 0.0; clamped to 0.0", value));
                *value = 0.0;
            } else if *value > 1.0 {
                warnings.push(format!("Value {} > 1.0; clamped to 1.0", value));
                *value = 1.0;
            }
        }
    }

    // Deduplicate SetParameter: keep last for each (track, plugin_index, param_index)
    let mut last_set_idx: HashMap<(i32, i32, i32), usize> = HashMap::new();
    for (idx, a) in actions.iter().enumerate() {
        if let ParameterAction::SetParameter {
            track,
            plugin_index,
            param_index,
            ..
        } = a
        {
            last_set_idx.insert((*track, *plugin_index, *param_index), idx);
        }
    }

    let mut filtered = Vec::with_capacity(actions.len());
    for (idx, a) in actions.into_iter().enumerate() {
        match &a {
            ParameterAction::SetParameter {
                track,
                plugin_index,
                param_index,
                ..
            } => {
                let key = (*track, *plugin_index, *param_index);
                if matches!(last_set_idx.get(&key), Some(last) if *last == idx) {
                    filtered.push(a);
                }
            }
            _ => filtered.push(a),
        }
    }

    // Deterministic ordering:
    // - Load -> Enable -> Set
    // - Within Set: "gate" params (enable/bypass) first
    let mut indexed: Vec<( (i32, i32, i32, usize), ParameterAction)> = filtered
        .into_iter()
        .enumerate()
        .map(|(idx, a)| {
            let type_rank = match &a {
                ParameterAction::LoadPlugin { .. } => 0,
                ParameterAction::EnablePlugin { .. } => 1,
                ParameterAction::SetParameter { .. } => 2,
            };

            let plugin_rank: i32 = match &a {
                ParameterAction::LoadPlugin { .. } => -1,
                ParameterAction::EnablePlugin { plugin_index, .. } => *plugin_index,
                ParameterAction::SetParameter { plugin_index, .. } => *plugin_index,
            };

            let set_rank: i32 = match &a {
                ParameterAction::SetParameter { param_name, .. } => {
                    let n = normalize_token(param_name);
                    if n.contains("bypass") || n.contains("enable") || n.contains("enabled") || n.contains("active") || n.ends_with("on") {
                        0
                    } else {
                        1
                    }
                }
                _ => 0,
            };

            ((type_rank, plugin_rank, set_rank, idx), a)
        })
        .collect();

    indexed.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));
    let filtered: Vec<ParameterAction> = indexed.into_iter().map(|(_, a)| a).collect();

    filtered
}

fn map_effect_group(
    track: i32,
    plugin: &ReaperPlugin,
    effect: &EffectParameters,
    actions: &mut Vec<ParameterAction>,
    warnings: &mut Vec<String>,
) {
    map_param_group(
        track,
        plugin,
        &effect.parameters,
        &format!("effect:{}", effect.effect_type),
        actions,
        warnings,
    );
}

fn map_param_group(
    track: i32,
    plugin: &ReaperPlugin,
    params: &HashMap<String, f64>,
    group: &str,
    actions: &mut Vec<ParameterAction>,
    warnings: &mut Vec<String>,
) {
    for (key, value) in params {
        let maybe_param = pick_best_param(plugin, key);
        let Some(param) = maybe_param else {
            warnings.push(format!(
                "Unmapped {} param '{}' for plugin '{}'",
                group, key, plugin.name
            ));
            continue;
        };

        actions.push(ParameterAction::SetParameter {
            track,
            plugin_index: plugin.index,
            param_index: param.index,
            param_name: param.name.clone(),
            value: *value,
            reason: format!("{} :: {} -> {}", group, key, param.name),
        });
    }
}

fn map_eq_reaeq(
    track: i32,
    plugin: &ReaperPlugin,
    eq: &HashMap<String, f64>,
    max_points: usize,
    actions: &mut Vec<ParameterAction>,
    warnings: &mut Vec<String>,
) {
    // Pick strongest EQ points by |dB|
    let mut points: Vec<(f64, f64)> = eq
        .iter()
        .filter_map(|(k, db)| parse_frequency_hz(k).map(|hz| (hz, *db)))
        .collect();

    points.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
    points.truncate(max_points);

    if points.is_empty() {
        warnings.push("EQ map: no parsable frequency keys found; skipped".to_string());
        return;
    }

    // Gather bands from param names: "Band N Freq" and "Band N Gain"
    let mut band_freq_param: HashMap<i32, &crate::parameter_ai::ReaperParameter> = HashMap::new();
    let mut band_gain_param: HashMap<i32, &crate::parameter_ai::ReaperParameter> = HashMap::new();

    for p in &plugin.parameters {
        if let Some(band) = parse_reaeq_band_number(&p.name) {
            let name_norm = normalize_token(&p.name);
            if name_norm.contains("freq") {
                band_freq_param.insert(band, p);
            } else if name_norm.contains("gain") {
                band_gain_param.insert(band, p);
            }
        }
    }

    if band_freq_param.is_empty() || band_gain_param.is_empty() {
        warnings.push(format!(
            "EQ map: '{}' does not look like ReaEQ band params; skipped",
            plugin.name
        ));
        return;
    }

    // Assign requested points to increasing band numbers (simple deterministic)
    let mut bands: Vec<i32> = band_freq_param.keys().copied().collect();
    bands.sort();

    for ((hz, db), band) in points.into_iter().zip(bands.into_iter()) {
        let Some(freq_param) = band_freq_param.get(&band) else { continue };
        let Some(gain_param) = band_gain_param.get(&band) else { continue };

        let freq_norm = hz_to_normalized_log(hz);
        let gain_norm = db_to_normalized(db, 24.0);

        actions.push(ParameterAction::SetParameter {
            track,
            plugin_index: plugin.index,
            param_index: freq_param.index,
            param_name: freq_param.name.clone(),
            value: freq_norm,
            reason: format!("eq :: set band {} freq to {:.0} Hz", band, hz),
        });

        actions.push(ParameterAction::SetParameter {
            track,
            plugin_index: plugin.index,
            param_index: gain_param.index,
            param_name: gain_param.name.clone(),
            value: gain_norm,
            reason: format!("eq :: set band {} gain to {:+.1} dB", band, db),
        });
    }
}

fn pick_best_plugin<'a>(
    snapshot: &'a ReaperSnapshot,
    keywords: &[Cow<'static, str>],
) -> Option<&'a ReaperPlugin> {
    let mut best: Option<(&ReaperPlugin, i32)> = None;
    for p in &snapshot.plugins {
        let score = score_text_against_keywords(&p.name, keywords);
        if score <= 0 {
            continue;
        }
        match best {
            None => best = Some((p, score)),
            Some((_, best_score)) if score > best_score => best = Some((p, score)),
            _ => {}
        }
    }
    best.map(|(p, _)| p)
}

fn pick_best_param<'a>(plugin: &'a ReaperPlugin, key: &str) -> Option<&'a crate::parameter_ai::ReaperParameter> {
    let key_norm = normalize_token(key);
    let synonyms = synonyms_for_key(&key_norm);
    let mut best: Option<(&crate::parameter_ai::ReaperParameter, i32)> = None;
    for p in &plugin.parameters {
        let score = score_param_name(&p.name, &key_norm, &synonyms);
        if score <= 0 {
            continue;
        }
        match best {
            None => best = Some((p, score)),
            Some((_, best_score)) if score > best_score => best = Some((p, score)),
            _ => {}
        }
    }
    best.map(|(p, _)| p)
}

fn score_param_name(param_name: &str, key_norm: &str, synonyms: &[Cow<'static, str>]) -> i32 {
    let p = normalize_token(param_name);
    if p == key_norm {
        return 100;
    }
    if p.contains(key_norm) {
        return 60;
    }
    for (i, s) in synonyms.iter().enumerate() {
        let s = s.as_ref();
        if p == s {
            return 90 - i as i32;
        }
        if p.contains(s) {
            return 50 - i as i32;
        }
    }
    0
}

fn score_text_against_keywords(text: &str, keywords: &[Cow<'static, str>]) -> i32 {
    let t = normalize_token(text);
    let mut score = 0;
    for (i, k) in keywords.iter().enumerate() {
        if t.contains(k.as_ref()) {
            score += 10 - i as i32;
        }
    }
    score
}

fn kws(list: &[&'static str]) -> Vec<Cow<'static, str>> {
    list.iter().map(|s| Cow::Borrowed(*s)).collect()
}

fn role_keywords_amp() -> Vec<Cow<'static, str>> {
    kws(&["neuraldsp", "archetype", "amp", "sim", "amplifier", "dist", "gain"])
}

fn role_keywords_eq() -> Vec<Cow<'static, str>> {
    kws(&["reaeq", "proq", "eq", "equalizer"])
}

fn role_keywords_gate() -> Vec<Cow<'static, str>> {
    kws(&["reagate", "gate", "noisegate", "noise"])
}

fn role_keywords_reverb() -> Vec<Cow<'static, str>> {
    kws(&["reaverbate", "reaverb", "reverb", "room", "hall"])
}

fn role_keywords_delay() -> Vec<Cow<'static, str>> {
    kws(&["readelay", "delay", "echo"])
}

fn role_keywords_for_effect(effect_type_norm: &str) -> Vec<Cow<'static, str>> {
    match effect_type_norm {
        "noisegate" | "gate" => role_keywords_gate(),
        "overdrive" | "od" => kws(&["overdrive", "od", "screamer", "drive"]),
        "distortion" | "dist" => kws(&["distortion", "dist", "fuzz"]),
        "compressor" | "comp" => kws(&["compressor", "comp"]),
        _ => vec![Cow::Owned(effect_type_norm.to_string())],
    }
}

fn default_plugin_for_effect(effect_type_norm: &str) -> Option<&'static str> {
    match effect_type_norm {
        "noisegate" | "gate" => Some("ReaGate (Cockos)"),
        _ => None,
    }
}

fn synonyms_for_key(key_norm: &str) -> Vec<Cow<'static, str>> {
    match key_norm {
        "gain" => kws(&["gain", "drive", "input", "pregain", "preamp"]),
        "drive" => kws(&["drive", "gain", "input"]),
        "bass" | "low" => kws(&["bass", "low", "lf", "lows"]),
        "mid" | "middle" => kws(&["mid", "middle", "mids", "mf"]),
        "treble" | "high" => kws(&["treble", "high", "hf", "highs", "presence"]),
        "presence" => kws(&["presence", "pres", "bright"]),
        "master" | "output" | "level" | "volume" => kws(&["master", "output", "level", "volume"]),
        "threshold" => kws(&["threshold", "thresh"]),
        "attack" => kws(&["attack", "att"]),
        "release" => kws(&["release", "rel"]),
        "mix" => kws(&["mix", "wet", "drywet", "blend"]),
        "time" => kws(&["time", "ms", "sec", "seconds"]),
        "feedback" => kws(&["feedback", "fb"]),
        _ => vec![Cow::Owned(key_norm.to_string())],
    }
}

fn normalize_token(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn contains_token(text: &str, token: &str) -> bool {
    normalize_token(text).contains(&normalize_token(token))
}

fn parse_frequency_hz(text: &str) -> Option<f64> {
    let s = text.trim().to_lowercase().replace(' ', "");
    if let Some(khz_pos) = s.find("khz") {
        let num = &s[..khz_pos];
        let v: f64 = num.parse().ok()?;
        return Some(v * 1000.0);
    }
    if let Some(hz_pos) = s.find("hz") {
        let num = &s[..hz_pos];
        let v: f64 = num.parse().ok()?;
        return Some(v);
    }
    None
}

fn parse_reaeq_band_number(param_name: &str) -> Option<i32> {
    let lower = param_name.to_lowercase();
    let band_pos = lower.find("band")?;
    let after = &lower[band_pos + 4..];
    let after = after.trim_start();
    let mut digits = String::new();
    for c in after.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    digits.parse().ok()
}

fn db_to_normalized(db: f64, max_abs_db: f64) -> f64 {
    let clamped = db.clamp(-max_abs_db, max_abs_db);
    (clamped + max_abs_db) / (2.0 * max_abs_db)
}

fn hz_to_normalized_log(hz: f64) -> f64 {
    let hz = hz.clamp(20.0, 20_000.0);
    let min = 20.0_f64.ln();
    let max = 20_000.0_f64.ln();
    (hz.ln() - min) / (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parameter_ai::{ReaperParameter, ReaperPlugin, ReaperSnapshot};

    fn fake_snapshot() -> ReaperSnapshot {
        ReaperSnapshot {
            track_index: 0,
            track_name: "Guitar".to_string(),
            plugins: vec![ReaperPlugin {
                index: 0,
                name: "VST3: Neural DSP Archetype Gojira".to_string(),
                enabled: true,
                parameters: vec![
                    ReaperParameter {
                        index: 0,
                        name: "Gain".to_string(),
                        current_value: 0.5,
                        display_value: "50%".to_string(),
                        unit: "%".to_string(),
                        format_hint: "percentage".to_string(),
                    },
                    ReaperParameter {
                        index: 1,
                        name: "Bass".to_string(),
                        current_value: 0.5,
                        display_value: "0.0 dB".to_string(),
                        unit: "dB".to_string(),
                        format_hint: "decibel".to_string(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn maps_amp_params_to_actions() {
        let snapshot = fake_snapshot();
        let mut params = ToneParameters {
            amp: HashMap::new(),
            eq: HashMap::new(),
            effects: vec![],
            reverb: HashMap::new(),
            delay: HashMap::new(),
        };
        params.amp.insert("gain".to_string(), 0.8);
        params.amp.insert("bass".to_string(), 0.7);

        let mapper = ChainMapper::new(ChainMapperConfig {
            allow_load_plugins: false,
            ..Default::default()
        });
        let result = mapper.map(&params, &snapshot);
        assert!(!result.actions.is_empty());
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, ParameterAction::SetParameter { param_name, .. } if param_name == "Gain")));
        assert!(result
            .actions
            .iter()
            .any(|a| matches!(a, ParameterAction::SetParameter { param_name, .. } if param_name == "Bass")));
    }

    #[test]
    fn parses_frequency_strings() {
        assert_eq!(parse_frequency_hz("800Hz").unwrap() as i32, 800);
        assert_eq!(parse_frequency_hz("2kHz").unwrap() as i32, 2000);
        assert!(parse_frequency_hz("abc").is_none());
    }
}
