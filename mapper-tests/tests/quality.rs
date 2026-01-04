use std::collections::HashMap;
use toneforge_mapper_tests::parameter_ai::{ParameterAction, ReaperParameter, ReaperPlugin, ReaperSnapshot};
use toneforge_mapper_tests::tone_encyclopedia::{EffectParameters, ToneParameters};
use toneforge_mapper_tests::{sanitize_tone, ChainMapper, ChainMapperConfig};

fn snapshot_with_plugins(plugins: Vec<ReaperPlugin>) -> ReaperSnapshot {
    ReaperSnapshot {
        track_index: 0,
        track_name: "Guitar".to_string(),
        plugins,
    }
}

fn param(index: i32, name: &str) -> ReaperParameter {
    ReaperParameter {
        index,
        name: name.to_string(),
        current_value: 0.5,
        display_value: "50%".to_string(),
        unit: "%".to_string(),
        format_hint: "percentage".to_string(),
    }
}

#[test]
fn orders_actions_load_then_enable_then_set() {
    let snapshot = snapshot_with_plugins(vec![ReaperPlugin {
        index: 0,
        name: "VST3: Neural DSP Archetype".to_string(),
        enabled: false,
        parameters: vec![param(0, "Gain")],
    }]);

    let mut tone = ToneParameters {
        amp: HashMap::new(),
        eq: HashMap::new(),
        effects: vec![EffectParameters {
            effect_type: "noise_gate".to_string(),
            parameters: HashMap::from([("threshold".to_string(), 0.3)]),
        }],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };
    tone.amp.insert("gain".to_string(), 0.9);

    let mapper = ChainMapper::new(ChainMapperConfig::default());
    let result = mapper.map(&tone, &snapshot);

    // With allow_load_plugins=true, noise gate missing -> LoadPlugin should appear (then enables/sets)
    let mut phase = 0;
    for a in &result.actions {
        let k = match a {
            ParameterAction::LoadPlugin { .. } => 0,
            ParameterAction::EnablePlugin { .. } => 1,
            ParameterAction::SetParameter { .. } => 2,
        };
        assert!(k >= phase, "actions not ordered: saw {:?} after phase {}", a, phase);
        phase = k;
    }
}

#[test]
fn clamps_out_of_range_values() {
    let snapshot = snapshot_with_plugins(vec![ReaperPlugin {
        index: 0,
        name: "VST3: Neural DSP Archetype".to_string(),
        enabled: true,
        parameters: vec![param(0, "Gain")],
    }]);

    let mut tone = ToneParameters {
        amp: HashMap::new(),
        eq: HashMap::new(),
        effects: vec![],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };
    tone.amp.insert("gain".to_string(), 999.0);

    let mapper = ChainMapper::new(ChainMapperConfig {
        allow_load_plugins: false,
        ..Default::default()
    });
    let result = mapper.map(&tone, &snapshot);

    let set = result.actions.iter().find_map(|a| match a {
        ParameterAction::SetParameter { value, .. } => Some(*value),
        _ => None,
    });
    assert_eq!(set, Some(1.0));
    assert!(!result.warnings.is_empty());
}

#[test]
fn picks_reaeq_for_eq_role_when_present() {
    let snapshot = snapshot_with_plugins(vec![
        ReaperPlugin {
            index: 0,
            name: "VST3: Neural DSP Archetype".to_string(),
            enabled: true,
            parameters: vec![param(0, "Gain")],
        },
        ReaperPlugin {
            index: 1,
            name: "ReaEQ (Cockos)".to_string(),
            enabled: true,
            parameters: vec![param(0, "Band 1 Freq"), param(1, "Band 1 Gain")],
        },
    ]);

    let tone = ToneParameters {
        amp: HashMap::new(),
        eq: HashMap::from([("800Hz".to_string(), -4.0)]),
        effects: vec![],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };

    let mapper = ChainMapper::new(ChainMapperConfig {
        allow_load_plugins: false,
        ..Default::default()
    });
    let result = mapper.map(&tone, &snapshot);

    assert!(result.actions.iter().any(|a| matches!(a, ParameterAction::SetParameter { plugin_index: 1, .. })));
}

#[test]
fn deterministic_for_same_input() {
    let snapshot = snapshot_with_plugins(vec![
        ReaperPlugin {
            index: 0,
            name: "VST3: Neural DSP Archetype".to_string(),
            enabled: true,
            parameters: vec![param(0, "Gain"), param(1, "Bass"), param(2, "Presence")],
        },
        ReaperPlugin {
            index: 1,
            name: "ReaEQ (Cockos)".to_string(),
            enabled: true,
            parameters: vec![param(0, "Band 1 Freq"), param(1, "Band 1 Gain")],
        },
    ]);

    let mut tone = ToneParameters {
        amp: HashMap::new(),
        eq: HashMap::new(),
        effects: vec![],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };
    tone.amp.insert("gain".to_string(), 0.75);
    tone.amp.insert("presence".to_string(), 0.6);
    tone.eq.insert("800Hz".to_string(), -4.0);

    let mapper = ChainMapper::new(ChainMapperConfig {
        allow_load_plugins: false,
        ..Default::default()
    });

    let a = mapper.map(&tone, &snapshot);
    let b = mapper.map(&tone, &snapshot);
    assert_eq!(a.actions, b.actions);
    assert_eq!(a.warnings, b.warnings);
    assert_eq!(a.summary, b.summary);
}

#[test]
fn warns_on_unmapped_param() {
    let snapshot = snapshot_with_plugins(vec![ReaperPlugin {
        index: 0,
        name: "VST3: Neural DSP Archetype".to_string(),
        enabled: true,
        parameters: vec![param(0, "Gain")],
    }]);

    let mut tone = ToneParameters {
        amp: HashMap::new(),
        eq: HashMap::new(),
        effects: vec![],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };
    tone.amp.insert("super_unknown_knob".to_string(), 0.42);

    let mapper = ChainMapper::new(ChainMapperConfig {
        allow_load_plugins: false,
        ..Default::default()
    });
    let result = mapper.map(&tone, &snapshot);

    assert!(!result.warnings.is_empty());
    assert!(result.actions.iter().all(|a| !matches!(a, ParameterAction::SetParameter { .. })));
}

#[test]
fn inserts_section_gate_toggle_before_setting_section_param() {
    let snapshot = snapshot_with_plugins(vec![ReaperPlugin {
        index: 0,
        name: "VST3: Amp Sim".to_string(),
        enabled: true,
        parameters: vec![
            ReaperParameter {
                index: 10,
                name: "EQ Bypass".to_string(),
                current_value: 1.0, // bypassed
                display_value: "On".to_string(),
                unit: "".to_string(),
                format_hint: "raw".to_string(),
            },
            ReaperParameter {
                index: 11,
                name: "EQ Gain".to_string(),
                current_value: 0.5,
                display_value: "0.0 dB".to_string(),
                unit: "dB".to_string(),
                format_hint: "decibel".to_string(),
            },
        ],
    }]);

    let mut tone = ToneParameters {
        amp: HashMap::new(),
        eq: HashMap::new(),
        effects: vec![],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };
    tone.amp.insert("gain".to_string(), 0.8);

    let mapper = ChainMapper::new(ChainMapperConfig {
        allow_load_plugins: false,
        ..Default::default()
    });
    let result = mapper.map(&tone, &snapshot);

    let mut seen_gate = false;
    let mut seen_target = false;
    for a in &result.actions {
        if let ParameterAction::SetParameter { param_index, value, .. } = a {
            if *param_index == 10 {
                seen_gate = true;
                assert_eq!(*value, 0.0);
            }
            if *param_index == 11 {
                seen_target = true;
                assert!(seen_gate, "target param set before gate was toggled");
            }
        }
    }

    assert!(seen_gate);
    assert!(seen_target);
}

#[test]
fn inserts_enable_plugin_if_plugin_disabled_but_params_set() {
    let snapshot = snapshot_with_plugins(vec![ReaperPlugin {
        index: 0,
        name: "VST3: Neural DSP Archetype".to_string(),
        enabled: false,
        parameters: vec![param(0, "Gain")],
    }]);

    let mut tone = ToneParameters {
        amp: HashMap::new(),
        eq: HashMap::new(),
        effects: vec![],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };
    tone.amp.insert("gain".to_string(), 0.7);

    let mapper = ChainMapper::new(ChainMapperConfig {
        allow_load_plugins: false,
        ..Default::default()
    });
    let result = mapper.map(&tone, &snapshot);

    let mut saw_enable = false;
    let mut saw_set = false;
    for a in &result.actions {
        match a {
            ParameterAction::EnablePlugin { plugin_index, .. } => {
                if *plugin_index == 0 {
                    saw_enable = true;
                }
            }
            ParameterAction::SetParameter { plugin_index, .. } => {
                if *plugin_index == 0 {
                    saw_set = true;
                    assert!(saw_enable, "SetParameter appeared before EnablePlugin");
                }
            }
            _ => {}
        }
    }

    assert!(saw_enable);
    assert!(saw_set);
}

#[test]
fn sanitizer_clamps_and_canonicalizes_engineer_output() {
    let mut tone = ToneParameters {
        amp: HashMap::from([
            ("Drive".to_string(), 2.0), // should canonicalize to gain and clamp
            ("TreB".to_string(), -1.0), // treble + clamp
        ]),
        eq: HashMap::from([
            ("800Hz".to_string(), -99.0),
            ("2kHz".to_string(), 99.0),
        ]),
        effects: vec![EffectParameters {
            effect_type: "Gate".to_string(),
            parameters: HashMap::from([("Thresh".to_string(), 5.0)]),
        }],
        reverb: HashMap::new(),
        delay: HashMap::new(),
    };

    // also add many eq points; sanitizer should cap
    for i in 0..50 {
        tone.eq.insert(format!("{}Hz", 100 + i), 0.1);
    }

    let sanitized = sanitize_tone(tone);

    assert!(sanitized.parameters.amp.contains_key("gain"));
    assert!(sanitized.parameters.amp.contains_key("treble"));
    assert!(sanitized.parameters.amp.get("gain").unwrap() <= &1.0);
    assert!(sanitized.parameters.amp.get("treble").unwrap() >= &0.0);
    assert!(sanitized.parameters.eq.len() <= 16);
    assert_eq!(sanitized.parameters.effects[0].effect_type, "noise_gate");
    assert!(sanitized.parameters.effects[0]
        .parameters
        .get("threshold")
        .is_some());
    assert!(!sanitized.warnings.is_empty());
}

#[test]
fn invariants_hold_across_varied_inputs() {
    // Lightweight deterministic pseudo-random sweep (no external deps).
    fn lcg(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *state
    }

    for seed in 0..50u64 {
        let mut state = seed ^ 0xA5A5_A5A5_A5A5_A5A5;

        let plugin_enabled = (lcg(&mut state) % 2) == 0;
        let bypass_on = (lcg(&mut state) % 2) == 0;

        let snapshot = snapshot_with_plugins(vec![ReaperPlugin {
            index: 0,
            name: "VST3: Amp Sim".to_string(),
            enabled: plugin_enabled,
            parameters: vec![
                ReaperParameter {
                    index: 10,
                    name: "EQ Bypass".to_string(),
                    current_value: if bypass_on { 1.0 } else { 0.0 },
                    display_value: if bypass_on { "On" } else { "Off" }.to_string(),
                    unit: "".to_string(),
                    format_hint: "raw".to_string(),
                },
                ReaperParameter {
                    index: 11,
                    name: "EQ Gain".to_string(),
                    current_value: 0.5,
                    display_value: "0.0 dB".to_string(),
                    unit: "dB".to_string(),
                    format_hint: "decibel".to_string(),
                },
            ],
        }]);

        let mut tone = ToneParameters {
            amp: HashMap::new(),
            eq: HashMap::new(),
            effects: vec![EffectParameters {
                effect_type: "Gate".to_string(),
                parameters: HashMap::from([("Thresh".to_string(), 2.5)]),
            }],
            reverb: HashMap::new(),
            delay: HashMap::new(),
        };
        // Intentionally out-of-range to test clamping.
        let raw_gain = (lcg(&mut state) as f64 / u64::MAX as f64) * 3.0 - 1.0;
        tone.amp.insert("gain".to_string(), raw_gain);

        let sanitized = sanitize_tone(tone);
        let mapper = ChainMapper::new(ChainMapperConfig {
            allow_load_plugins: false,
            ..Default::default()
        });
        let result = mapper.map(&sanitized.parameters, &snapshot);

        // Invariant: values are clamped to [0, 1]
        for a in &result.actions {
            if let ParameterAction::SetParameter { value, .. } = a {
                assert!(*value >= 0.0 && *value <= 1.0);
            }
        }

        // Invariant: action order Load -> Enable -> Set
        let mut phase = 0;
        for a in &result.actions {
            let k = match a {
                ParameterAction::LoadPlugin { .. } => 0,
                ParameterAction::EnablePlugin { .. } => 1,
                ParameterAction::SetParameter { .. } => 2,
            };
            assert!(k >= phase);
            phase = k;
        }

        // Invariant: if plugin disabled in snapshot, an enable must appear before sets to it
        if !plugin_enabled {
            let mut saw_enable = false;
            for a in &result.actions {
                match a {
                    ParameterAction::EnablePlugin { plugin_index, .. } if *plugin_index == 0 => {
                        saw_enable = true
                    }
                    ParameterAction::SetParameter { plugin_index, .. } if *plugin_index == 0 => {
                        assert!(saw_enable, "seed {}: set before enable", seed);
                    }
                    _ => {}
                }
            }
        }

        // Invariant: if bypass is on and we set EQ Gain, gate toggle should be present.
        if bypass_on {
            let sets_eq_gain = result.actions.iter().any(|a| matches!(a, ParameterAction::SetParameter { param_index: 11, .. }));
            if sets_eq_gain {
                assert!(result.actions.iter().any(|a| matches!(a, ParameterAction::SetParameter { param_index: 10, value, .. } if (*value - 0.0).abs() < f64::EPSILON)));
            }
        }
    }
}
