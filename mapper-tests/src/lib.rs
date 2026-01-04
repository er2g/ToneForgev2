pub mod parameter_ai {
    #[derive(Debug, Clone, PartialEq)]
    pub struct ReaperSnapshot {
        pub track_index: i32,
        pub track_name: String,
        pub plugins: Vec<ReaperPlugin>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ReaperPlugin {
        pub index: i32,
        pub name: String,
        pub enabled: bool,
        pub parameters: Vec<ReaperParameter>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ReaperParameter {
        pub index: i32,
        pub name: String,
        pub current_value: f64,
        pub display_value: String,
        pub unit: String,
        pub format_hint: String,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub enum ParameterAction {
        SetParameter {
            track: i32,
            plugin_index: i32,
            param_index: i32,
            param_name: String,
            value: f64,
            reason: String,
        },
        EnablePlugin {
            track: i32,
            plugin_index: i32,
            plugin_name: String,
            reason: String,
        },
        LoadPlugin {
            track: i32,
            plugin_name: String,
            position: Option<i32>,
            reason: String,
        },
    }
}

pub mod tone_encyclopedia {
    use std::collections::HashMap;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ToneParameters {
        pub amp: HashMap<String, f64>,
        pub eq: HashMap<String, f64>,
        pub effects: Vec<EffectParameters>,
        pub reverb: HashMap<String, f64>,
        pub delay: HashMap<String, f64>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct EffectParameters {
        pub effect_type: String,
        pub parameters: HashMap<String, f64>,
    }
}

// Compile the exact mapper code under test.
#[path = "../../tauri-app/src-tauri/src/chain_mapper.rs"]
mod chain_mapper;

pub use chain_mapper::{ChainMapper, ChainMapperConfig, ChainMappingResult};

#[path = "../../tauri-app/src-tauri/src/tone_sanitizer.rs"]
mod tone_sanitizer;

pub use tone_sanitizer::{sanitize as sanitize_tone, SanitizedTone};
