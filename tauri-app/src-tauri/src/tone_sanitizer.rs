//! Tone parameter sanitization for safe, predictable mapping.
//!
//! Purpose:
//! - Clamp values to expected ranges
//! - Remove NaN/Inf
//! - Canonicalize common key/effect synonyms
//! - Cap list sizes so downstream mapping stays deterministic

use crate::tone_encyclopedia::{EffectParameters, ToneParameters};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SanitizedTone {
    pub parameters: ToneParameters,
    pub warnings: Vec<String>,
}

pub fn sanitize(mut parameters: ToneParameters) -> SanitizedTone {
    let mut warnings = Vec::new();

    parameters.amp = sanitize_unit_map(parameters.amp, "amp", &mut warnings, 32);
    parameters.reverb = sanitize_unit_map(parameters.reverb, "reverb", &mut warnings, 32);
    parameters.delay = sanitize_unit_map(parameters.delay, "delay", &mut warnings, 32);

    parameters.eq = sanitize_eq_map(parameters.eq, &mut warnings, 16);
    parameters.effects = sanitize_effects(parameters.effects, &mut warnings, 5, 24);

    SanitizedTone { parameters, warnings }
}

fn sanitize_effects(
    mut effects: Vec<EffectParameters>,
    warnings: &mut Vec<String>,
    max_effects: usize,
    max_params_per_effect: usize,
) -> Vec<EffectParameters> {
    if effects.len() > max_effects {
        warnings.push(format!(
            "effects: {} entries provided; keeping first {}",
            effects.len(),
            max_effects
        ));
        effects.truncate(max_effects);
    }

    for eff in &mut effects {
        let original_type = eff.effect_type.clone();
        eff.effect_type = canonical_effect_type(&eff.effect_type);
        if eff.effect_type != original_type {
            warnings.push(format!(
                "effects: normalized effect_type '{}' -> '{}'",
                original_type, eff.effect_type
            ));
        }
        eff.parameters = sanitize_unit_map(
            std::mem::take(&mut eff.parameters),
            &format!("effect:{}", eff.effect_type),
            warnings,
            max_params_per_effect,
        );
    }

    let before = effects.len();
    effects.retain(|e| !e.parameters.is_empty());
    if effects.len() != before {
        warnings.push(format!(
            "effects: dropped {} empty effect(s) after sanitization",
            before - effects.len()
        ));
    }

    effects
}

fn sanitize_unit_map(
    map: HashMap<String, f64>,
    group: &str,
    warnings: &mut Vec<String>,
    max_keys: usize,
) -> HashMap<String, f64> {
    let mut out = HashMap::new();

    for (k, v) in map {
        if !v.is_finite() {
            warnings.push(format!("{}: dropped non-finite value for '{}'", group, k));
            continue;
        }

        let canonical_key = match canonical_param_key(group, &k) {
            Some(v) => v,
            None => {
                if is_strict_group(group) {
                    warnings.push(format!(
                        "{}: dropped unsupported key '{}' (strict vocabulary)",
                        group, k
                    ));
                    continue;
                }
                k.clone()
            }
        };
        let clamped = v.clamp(0.0, 1.0);
        if (clamped - v).abs() > f64::EPSILON {
            warnings.push(format!(
                "{}: clamped '{}' from {:.3} to {:.3}",
                group, canonical_key, v, clamped
            ));
        }

        if out.len() < max_keys {
            out.insert(canonical_key, clamped);
        }
    }

    if out.len() > max_keys {
        warnings.push(format!("{}: too many keys; capped to {}", group, max_keys));
    }

    out
}

fn is_strict_group(group: &str) -> bool {
    if group == "delay" || group == "reverb" {
        return true;
    }
    group.starts_with("effect:")
}

fn sanitize_eq_map(mut map: HashMap<String, f64>, warnings: &mut Vec<String>, max_points: usize) -> HashMap<String, f64> {
    map.retain(|k, v| {
        if !v.is_finite() {
            warnings.push(format!("eq: dropped non-finite value for '{}'", k));
            return false;
        }
        true
    });

    for (k, v) in map.iter_mut() {
        let clamped = v.clamp(-12.0, 12.0);
        if (clamped - *v).abs() > f64::EPSILON {
            warnings.push(format!("eq: clamped '{}' from {:+.1} dB to {:+.1} dB", k, *v, clamped));
        }
        *v = clamped;
    }

    if map.len() <= max_points {
        return map;
    }

    let mut pairs: Vec<(String, f64)> = map.into_iter().collect();
    pairs.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(max_points);

    warnings.push(format!(
        "eq: too many points; keeping top {} by |dB|",
        max_points
    ));

    pairs.into_iter().collect()
}

fn canonical_effect_type(effect_type: &str) -> String {
    let t = normalize_token(effect_type);
    match t.as_str() {
        "noisegate" | "gate" | "noise_gate" => "noise_gate".to_string(),
        "overdrive" | "od" | "tubescreamer" | "screamer" => "overdrive".to_string(),
        "distortion" | "dist" | "fuzz" => "distortion".to_string(),
        "compressor" | "comp" => "compressor".to_string(),
        "chorus" => "chorus".to_string(),
        "phaser" => "phaser".to_string(),
        _ => effect_type.to_string(),
    }
}

fn canonical_param_key(group: &str, key: &str) -> Option<String> {
    let k = normalize_token(key);
    if group == "amp" {
        return Some(match k.as_str() {
            "gain" | "drive" | "input" | "pregain" | "preamp" => "gain",
            "bass" | "low" | "lows" => "bass",
            "mid" | "middle" | "mids" => "mid",
            "treble" | "treb" | "high" | "highs" => "treble",
            "presence" | "pres" | "bright" => "presence",
            "master" | "volume" | "level" | "output" => "master",
            _ => return None,
        }
        .to_string());
    }

    if let Some(effect_type) = group.strip_prefix("effect:") {
        let et = normalize_token(effect_type);
        if et == "noise_gate" || et == "noisegate" || et == "gate" {
            return Some(match k.as_str() {
                "threshold" | "thresh" => "threshold",
                "attack" | "att" => "attack",
                // Many gate UIs label this as "decay"; map to release to keep vocabulary small.
                "release" | "rel" | "decay" => "release",
                _ => return None,
            }
            .to_string());
        }

        if et == "compressor" || et == "comp" {
            return Some(match k.as_str() {
                "threshold" | "thresh" => "threshold",
                "attack" | "att" => "attack",
                "release" | "rel" => "release",
                "ratio" => "ratio",
                "mix" | "wet" | "drywet" | "blend" => "mix",
                "makeup" | "makeupgain" | "gain" | "output" | "level" => "makeup",
                _ => return None,
            }
            .to_string());
        }

        if et == "overdrive" || et == "od" {
            return Some(match k.as_str() {
                "drive" | "gain" => "drive",
                "tone" | "treble" => "tone",
                "level" | "output" | "volume" => "level",
                _ => return None,
            }
            .to_string());
        }

        if et == "distortion" || et == "dist" || et == "fuzz" {
            return Some(match k.as_str() {
                "drive" | "gain" => "drive",
                "tone" => "tone",
                "level" | "output" | "volume" => "level",
                "low" | "lows" | "bass" => "low",
                "high" | "highs" | "treble" => "high",
                _ => return None,
            }
            .to_string());
        }

        if et == "chorus" {
            return Some(match k.as_str() {
                "rate" => "rate",
                "depth" => "depth",
                "mix" | "wet" | "drywet" | "blend" => "mix",
                _ => return None,
            }
            .to_string());
        }
    }

    if group == "reverb" {
        return Some(match k.as_str() {
            "mix" | "wet" | "drywet" | "blend" => "mix",
            "roomsize" | "room_size" | "size" => "room_size",
            "predelay" | "pre_delay" | "pre" => "predelay",
            "decay" | "time" => "decay",
            "highcut" | "high_cut" | "hicut" => "high_cut",
            "lowcut" | "low_cut" | "locut" => "low_cut",
            _ => return None,
        }
        .to_string());
    }

    if group == "delay" {
        return Some(match k.as_str() {
            "mix" | "wet" | "drywet" | "blend" => "mix",
            "time" | "ms" | "seconds" | "sec" => "time",
            "feedback" | "fb" => "feedback",
            _ => return None,
        }
        .to_string());
    }

    Some(match k.as_str() {
        "mix" | "wet" | "drywet" | "blend" => "mix",
        "time" | "ms" | "seconds" | "sec" => "time",
        "feedback" | "fb" => "feedback",
        "threshold" | "thresh" => "threshold",
        "attack" | "att" => "attack",
        "release" | "rel" => "release",
        _ => return None,
    }
    .to_string())
}

fn normalize_token(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}
