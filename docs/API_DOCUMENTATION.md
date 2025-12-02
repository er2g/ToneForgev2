# ToneForge API Documentation

Version: 2.0
Last Updated: December 2, 2025

---

## Table of Contents

1. [REAPER Extension HTTP API](#reaper-extension-http-api)
2. [Tauri Commands API](#tauri-commands-api)
3. [AI Engine API](#ai-engine-api)
4. [Audio Analysis API](#audio-analysis-api)
5. [Type Definitions](#type-definitions)
6. [Error Handling](#error-handling)

---

## REAPER Extension HTTP API

The REAPER extension runs an HTTP server on `http://127.0.0.1:8888` providing REST endpoints for controlling REAPER.

### Base URL
```
http://127.0.0.1:8888
```

### Authentication
No authentication required (localhost only).

---

### 1. Health Check

#### `GET /ping`

Health check endpoint to verify the extension is running.

**Response:**
```json
{
  "status": "ok",
  "service": "ToneForge REAPER Extension"
}
```

**Example:**
```bash
curl http://127.0.0.1:8888/ping
```

---

### 2. Track Management

#### `GET /tracks`

Get overview of all tracks and their FX states.

**Response:**
```json
{
  "track_count": 2,
  "tracks": [
    {
      "index": 0,
      "name": "Track 1",
      "fx_count": 2,
      "fx_list": [
        {
          "index": 0,
          "name": "VST: Neural DSP Gojira",
          "enabled": true
        },
        {
          "index": 1,
          "name": "VST: FabFilter Pro-Q 3",
          "enabled": false
        }
      ]
    }
  ]
}
```

**Example:**
```bash
curl http://127.0.0.1:8888/tracks
```

---

### 3. FX Management

#### `GET /fx/list`

Get FX list for a specific track.

**Query Parameters:**
- `track` (optional, default: 0): Track index

**Response:**
```json
{
  "track": 0,
  "fx_count": 2,
  "fx_list": [
    {
      "index": 0,
      "name": "VST: Neural DSP Gojira"
    },
    {
      "index": 1,
      "name": "VST: FabFilter Pro-Q 3"
    }
  ]
}
```

**Example:**
```bash
curl "http://127.0.0.1:8888/fx/list?track=0"
```

---

#### `GET /fx/catalog`

Enumerate all installed FX plugins and their default parameter states.

**Query Parameters:**
- `refresh` (optional, default: 0): Set to 1 to force cache refresh

**Response:**
```json
{
  "count": 150,
  "cache_size": 150,
  "refreshed": false,
  "plugins": [
    {
      "name": "VST: Neural DSP Gojira",
      "format": "VST",
      "param_count": 45,
      "params": [
        {
          "index": 0,
          "name_raw": "Gain",
          "name_normalized": "gain",
          "default_normalized": 0.5
        }
      ]
    }
  ]
}
```

**Example:**
```bash
# Get cached catalog
curl http://127.0.0.1:8888/fx/catalog

# Force refresh
curl "http://127.0.0.1:8888/fx/catalog?refresh=1"
```

**Note:** First call may take several minutes as it instantiates and scans all plugins. Results are cached for subsequent calls.

---

#### `GET /fx/params`

Get all parameters for a specific FX.

**Query Parameters:**
- `track` (optional, default: 0): Track index
- `fx` (optional, default: 0): FX index

**Response:**
```json
{
  "track": 0,
  "fx": 0,
  "params": [
    {
      "index": 0,
      "name": "Gain",
      "value": 0.75,
      "display": "6.2 dB",
      "unit": "dB",
      "format_hint": "decibel"
    },
    {
      "index": 1,
      "name": "Bass",
      "value": 0.60,
      "display": "432 Hz",
      "unit": "Hz",
      "format_hint": "frequency"
    }
  ]
}
```

**Example:**
```bash
curl "http://127.0.0.1:8888/fx/params?track=0&fx=0"
```

---

#### `POST /fx/add`

Add a plugin to a track.

**Request Body:**
```json
{
  "track": 0,
  "plugin": "VST: Neural DSP Gojira"
}
```

**Response:**
```json
{
  "success": true,
  "track": 0,
  "fx_index": 2,
  "fx_name": "VST: Neural DSP Gojira (Neural DSP)"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8888/fx/add \
  -H "Content-Type: application/json" \
  -d '{"track": 0, "plugin": "VST: Neural DSP Gojira"}'
```

**Error Response (404):**
```json
{
  "error": "Failed to load plugin",
  "plugin": "VST: InvalidPlugin"
}
```

---

#### `DELETE /fx/remove`

Remove an FX from a track.

**Query Parameters:**
- `track`: Track index
- `fx`: FX index

**Response:**
```json
{
  "success": true,
  "track": 0,
  "fx": 1
}
```

**Example:**
```bash
curl -X DELETE "http://127.0.0.1:8888/fx/remove?track=0&fx=1"
```

---

#### `POST /fx/toggle`

Toggle FX enable/bypass state.

**Request Body:**
```json
{
  "track": 0,
  "fx": 0,
  "enabled": true
}
```

**Response:**
```json
{
  "success": true,
  "track": 0,
  "fx": 0,
  "enabled": true
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8888/fx/toggle \
  -H "Content-Type: application/json" \
  -d '{"track": 0, "fx": 0, "enabled": false}'
```

---

### 4. Parameter Control

#### `GET /fx/param`

Get a single FX parameter value.

**Query Parameters:**
- `track`: Track index
- `fx`: FX index
- `param`: Parameter name (fuzzy matching, case-insensitive)

**Response:**
```json
{
  "track": 0,
  "fx": 0,
  "param": "gain",
  "param_index": 5,
  "value": 0.75
}
```

**Example:**
```bash
curl "http://127.0.0.1:8888/fx/param?track=0&fx=0&param=gain"
```

**Error Response (404):**
```json
{
  "error": "Parameter not found"
}
```

---

#### `POST /fx/param`

Set an FX parameter value.

**Request Body:**
```json
{
  "track": 0,
  "fx": 0,
  "param": "gain",
  "value": 0.75
}
```

**Parameters:**
- `track`: Track index
- `fx`: FX index
- `param`: Parameter name (fuzzy matching supported)
- `value`: Normalized value (0.0 - 1.0)

**Response:**
```json
{
  "success": true,
  "track": 0,
  "fx": 0,
  "param_index": 5,
  "value": 0.75
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8888/fx/param \
  -H "Content-Type: application/json" \
  -d '{
    "track": 0,
    "fx": 0,
    "param": "gain",
    "value": 0.75
  }'
```

**Error Response (404):**
```json
{
  "error": "Parameter not found",
  "searched": "invalid_param",
  "available_params": ["gain", "bass", "mid", "treble"]
}
```

---

### 5. Transport Control

#### `GET /transport/bpm`

Get current project BPM.

**Response:**
```json
{
  "bpm": 120.0,
  "beats_per_measure": 4.0
}
```

**Example:**
```bash
curl http://127.0.0.1:8888/transport/bpm
```

---

#### `POST /transport/bpm`

Set project BPM.

**Request Body:**
```json
{
  "bpm": 140.0
}
```

**Response:**
```json
{
  "success": true,
  "bpm": 140.0
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8888/transport/bpm \
  -H "Content-Type: application/json" \
  -d '{"bpm": 140.0}'
```

---

### 6. Project Management

#### `POST /project/save`

Save the current project.

**Request Body:**
```json
{
  "name": "my_preset"
}
```

**Response:**
```json
{
  "success": true,
  "preset_name": "my_preset",
  "project_path": "C:\\Users\\User\\Documents\\REAPER\\Projects\\my_preset.rpp"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8888/project/save \
  -H "Content-Type: application/json" \
  -d '{"name": "my_preset"}'
```

---

#### `POST /project/load`

Load a project.

**Request Body:**
```json
{
  "path": "C:\\Users\\User\\Documents\\REAPER\\Projects\\my_preset.rpp"
}
```

**Response:**
```json
{
  "success": true,
  "loaded_path": "C:\\Users\\User\\Documents\\REAPER\\Projects\\my_preset.rpp"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8888/project/load \
  -H "Content-Type: application/json" \
  -d '{"path": "C:\\\\path\\\\to\\\\project.rpp"}'
```

**Error Response (400):**
```json
{
  "error": "Project path required"
}
```

---

## Tauri Commands API

Tauri commands are invoked from the frontend JavaScript/TypeScript code using the `@tauri-apps/api` package.

### Installation

```typescript
import { invoke } from '@tauri-apps/api/core';
```

---

### 1. REAPER Connection

#### `check_reaper_connection`

Check if REAPER extension is running and accessible.

**Parameters:** None

**Returns:** `Promise<boolean>`

**Example:**
```typescript
const isConnected = await invoke<boolean>('check_reaper_connection');
console.log('REAPER connected:', isConnected);
```

**Error Response:**
```typescript
// Throws error string if connection fails
try {
  await invoke('check_reaper_connection');
} catch (error) {
  console.error('Connection error:', error);
}
```

---

### 2. AI Configuration

#### `configure_ai_provider`

Configure the AI provider (Gemini).

**Parameters:**
```typescript
{
  provider: string;  // "gemini"
  model: string;     // e.g., "gemini-1.5-pro"
  api_key: string;   // Your API key
}
```

**Returns:** `Promise<string>` - Success message

**Example:**
```typescript
const result = await invoke<string>('configure_ai_provider', {
  provider: 'gemini',
  model: 'gemini-1.5-pro',
  apiKey: 'your-api-key-here'
});
// Returns: "gemini configured (model: gemini-1.5-pro)"
```

---

#### `get_chat_history`

Get the chat history as JSON.

**Parameters:** None

**Returns:** `Promise<string>` - JSON string of chat messages

**Example:**
```typescript
const historyJson = await invoke<string>('get_chat_history');
const history = JSON.parse(historyJson);
// history: Array<{role: string, content: string, timestamp: number}>
```

---

#### `process_chat_message`

Send a message to the AI assistant for tone adjustment.

**Parameters:**
```typescript
{
  message: string;      // User message
  track: number | null; // Track index (optional, default: 0)
}
```

**Returns:** `Promise<string>` - JSON response

**Response Structure:**
```typescript
{
  summary: string;
  changes_table: Array<{
    plugin: string;
    parameter: string;
    old_value: string;
    new_value: string;
    reason: string;
  }>;
}
```

**Example:**
```typescript
const responseJson = await invoke<string>('process_chat_message', {
  message: 'Make the tone heavier with more gain',
  track: 0
});

const response = JSON.parse(responseJson);
console.log('Summary:', response.summary);
console.log('Changes:', response.changes_table);
```

**Example Response:**
```json
{
  "summary": "Increased the gain and adjusted the EQ for a heavier tone. I boosted the gain to 85%, reduced the bass slightly to prevent muddiness, and increased the mid presence for more aggression.",
  "changes_table": [
    {
      "plugin": "Neural DSP Gojira",
      "parameter": "Gain",
      "old_value": "50%",
      "new_value": "85%",
      "reason": "Increase distortion for heavier tone"
    },
    {
      "plugin": "Neural DSP Gojira",
      "parameter": "Bass",
      "old_value": "60%",
      "new_value": "50%",
      "reason": "Reduce bass to prevent muddiness with high gain"
    }
  ]
}
```

---

### 3. Track Overview

#### `get_track_overview`

Get overview of all tracks and FX.

**Parameters:** None

**Returns:** `Promise<string>` - JSON string

**Response Structure:**
```typescript
{
  track_count: number;
  tracks: Array<{
    index: number;
    name: string;
    fx_count: number;
    fx_list: Array<{
      index: number;
      name: string;
      enabled: boolean;
    }>;
  }>;
}
```

**Example:**
```typescript
const overviewJson = await invoke<string>('get_track_overview');
const overview = JSON.parse(overviewJson);
```

---

#### `set_fx_enabled`

Enable or disable (bypass) an FX.

**Parameters:**
```typescript
{
  track: number;
  fx: number;
  enabled: boolean;
}
```

**Returns:** `Promise<boolean>` - New enabled state

**Example:**
```typescript
const newState = await invoke<boolean>('set_fx_enabled', {
  track: 0,
  fx: 1,
  enabled: false
});
// Returns: false (FX is now bypassed)
```

---

### 4. Preset Management

#### `save_preset`

Save current project as a preset.

**Parameters:**
```typescript
{
  name: string;  // Preset name
}
```

**Returns:** `Promise<string>` - Full path to saved project

**Example:**
```typescript
const path = await invoke<string>('save_preset', {
  name: 'heavy_metal_tone'
});
// Returns: "C:\\Users\\User\\Documents\\REAPER\\Projects\\heavy_metal_tone.rpp"
```

---

#### `load_preset`

Load a preset from disk.

**Parameters:**
```typescript
{
  path: string;  // Full path to .rpp file
}
```

**Returns:** `Promise<string>` - Success message

**Example:**
```typescript
const result = await invoke<string>('load_preset', {
  path: 'C:\\Users\\User\\Documents\\REAPER\\Projects\\my_preset.rpp'
});
// Returns: "Preset loaded from C:\\Users\\User\\Documents\\REAPER\\Projects\\my_preset.rpp"
```

---

### 5. Audio Analysis

#### `load_reference_audio`

Load and analyze a reference audio file.

**Parameters:**
```typescript
{
  path: string;  // Full path to audio file
}
```

**Returns:** `Promise<EQProfile>`

**Response Structure:**
```typescript
{
  bands: Array<{
    frequency: number;      // Hz
    gain_db: number;       // dB
    bandwidth: number;     // Hz
    confidence: number;    // 0.0 - 1.0
  }>;
  overall_loudness: number;   // dB
  dynamic_range: number;      // dB
  spectral_centroid: number;  // Hz
  spectral_rolloff: number;   // Hz
}
```

**Example:**
```typescript
const profile = await invoke<EQProfile>('load_reference_audio', {
  path: 'C:\\Audio\\reference.wav'
});
console.log('Loudness:', profile.overall_loudness, 'dB');
```

---

#### `load_input_audio`

Load and analyze input audio file (same as reference).

**Parameters:**
```typescript
{
  path: string;
}
```

**Returns:** `Promise<EQProfile>`

**Example:**
```typescript
const inputProfile = await invoke<EQProfile>('load_input_audio', {
  path: 'C:\\Audio\\input.wav'
});
```

---

#### `calculate_eq_match`

Calculate EQ correction to match reference.

**Parameters:**
```typescript
{
  reference: EQProfile;
  input: EQProfile;
  config: {
    intensity: number;           // 0.0 - 1.0
    max_correction: number;      // Max ±dB per band
    smoothing_factor: number;    // 0.0 - 1.0
    use_psychoacoustic: boolean;
    preserve_dynamics: boolean;
  };
}
```

**Returns:** `Promise<MatchResult>`

**Response Structure:**
```typescript
{
  correction_profile: EQProfile;
  reference_normalized: number[];
  input_normalized: number[];
  quality_score: number;      // 0.0 - 1.0
  warnings: string[];
}
```

**Example:**
```typescript
const matchResult = await invoke<MatchResult>('calculate_eq_match', {
  reference: referenceProfile,
  input: inputProfile,
  config: {
    intensity: 0.7,
    max_correction: 6.0,
    smoothing_factor: 0.5,
    use_psychoacoustic: true,
    preserve_dynamics: true
  }
});

console.log('Quality:', matchResult.quality_score);
console.log('Warnings:', matchResult.warnings);
```

---

#### `export_eq_settings`

Export EQ settings to various formats.

**Parameters:**
```typescript
{
  result: MatchResult;
  format: "reaper" | "json" | "txt";
}
```

**Returns:** `Promise<string>` - Formatted output

**Example:**
```typescript
// Export as REAPER preset
const reaperPreset = await invoke<string>('export_eq_settings', {
  result: matchResult,
  format: 'reaper'
});

// Export as JSON
const jsonExport = await invoke<string>('export_eq_settings', {
  result: matchResult,
  format: 'json'
});

// Export as text
const textExport = await invoke<string>('export_eq_settings', {
  result: matchResult,
  format: 'txt'
});
```

**Text Format Output:**
```
EQ Settings:

    31 Hz:  +2.50 dB (Q: 2.10)
    63 Hz:  +1.80 dB (Q: 2.10)
   125 Hz:  +0.50 dB (Q: 2.10)
   250 Hz:  -1.20 dB (Q: 2.10)
   500 Hz:  -0.80 dB (Q: 2.10)
  1000 Hz:  +2.30 dB (Q: 2.10)
  2000 Hz:  +3.50 dB (Q: 2.10)
  4000 Hz:  +1.50 dB (Q: 2.10)
  8000 Hz:  -2.00 dB (Q: 2.10)
 16000 Hz:  -3.50 dB (Q: 2.10)
```

---

## AI Engine API

Professional-grade algorithms for audio parameter optimization.

### Overview

The AI Engine provides:
- State diffing (detect what changed)
- Action optimization (merge, deduplicate, reorder)
- Semantic parameter grouping
- Safety validation (bounds checking, conflict detection)
- Parameter relationship modeling
- Transaction support (rollback capability)

---

### 1. Semantic Analysis

#### `SemanticAnalyzer::categorize`

Categorize a parameter by name.

**Signature:**
```rust
pub fn categorize(param_name: &str) -> ParameterCategory
```

**Parameter Categories:**
- `Distortion`: gain, drive, overdrive, saturation
- `EQ`: bass, mid, treble, low, high
- `Dynamics`: compression, threshold, ratio
- `Modulation`: chorus, flanger, phaser, rate, depth
- `Delay`: delay time, feedback, mix
- `Reverb`: room size, decay, damping
- `Filter`: cutoff, resonance, Q
- `Volume`: level, output, mix
- `Toggle`: on/off switches, bypasses
- `Unknown`: uncategorized

**Example:**
```rust
use ai_engine::SemanticAnalyzer;

let category = SemanticAnalyzer::categorize("Gain");
// Returns: ParameterCategory::Distortion

let category = SemanticAnalyzer::categorize("Bass");
// Returns: ParameterCategory::EQ
```

---

### 2. Action Optimization

#### `ActionOptimizer::deduplicate`

Remove duplicate actions, keeping only the last modification.

**Signature:**
```rust
pub fn deduplicate(actions: Vec<ActionPlan>) -> Vec<ActionPlan>
```

**Example:**
```rust
use ai_engine::{ActionOptimizer, ActionPlan};

let actions = vec![
    ActionPlan {
        track: 0,
        fx_index: 0,
        param_index: 1,
        value: 0.5,
        reason: "First".to_string(),
    },
    ActionPlan {
        track: 0,
        fx_index: 0,
        param_index: 1,
        value: 0.8,  // Same param, different value
        reason: "Second".to_string(),
    },
];

let deduplicated = ActionOptimizer::deduplicate(actions);
// Result: Only the second action (value: 0.8) is kept
```

---

#### `ActionOptimizer::detect_conflicts`

Detect conflicting actions (same parameter set to multiple values).

**Signature:**
```rust
pub fn detect_conflicts(actions: &[ActionPlan]) -> Vec<String>
```

**Returns:** Vec of warning messages

**Example:**
```rust
let conflicts = ActionOptimizer::detect_conflicts(&actions);
for conflict in conflicts {
    println!("⚠️  {}", conflict);
}
// Output: "Conflict detected: Track 0 FX 0 Param 1 set to multiple values: [0.5, 0.8]"
```

---

#### `ActionOptimizer::reorder`

Reorder actions for optimal execution (toggles first, then parameters).

**Signature:**
```rust
pub fn reorder(actions: Vec<ActionPlan>) -> Vec<ActionPlan>
```

**Example:**
```rust
let optimized = ActionOptimizer::reorder(actions);
// Result: Enable/disable actions executed first, then parameter changes
```

---

### 3. Safety Validation

#### `SafetyValidator::validate_value`

Validate and clamp parameter value to safe range.

**Signature:**
```rust
pub fn validate_value(
    param_name: &str,
    value: f64,
) -> (f64, Option<String>)
```

**Returns:**
- `f64`: Clamped value
- `Option<String>`: Warning message if any

**Safety Bounds:**
- **Distortion**: 0.0 - 1.0, recommended max: 0.9
- **EQ**: 0.0 - 1.0, recommended max: 0.85
- **Volume**: 0.0 - 1.0, recommended max: 0.95
- **Others**: 0.0 - 1.0, recommended max: 1.0

**Example:**
```rust
use ai_engine::SafetyValidator;

let (clamped, warning) = SafetyValidator::validate_value("Gain", 1.5);
// clamped = 1.0 (clamped to max)
// warning = Some("Value 1.5 above maximum 1.0, clamping")

let (clamped, warning) = SafetyValidator::validate_value("Gain", 0.95);
// clamped = 0.95
// warning = Some("⚠️  Value 0.95 exceeds recommended max 0.9 for Distortion. May cause clipping/distortion.")
```

---

### 4. Parameter Relationships

#### `RelationshipEngine::suggest_compensations`

Suggest compensatory adjustments based on parameter relationships.

**Signature:**
```rust
pub fn suggest_compensations(
    param_name: &str,
    old_value: f64,
    new_value: f64,
) -> Vec<(String, f64, String)>
```

**Returns:** Vec of `(parameter_name, suggested_delta, reason)`

**Example:**
```rust
use ai_engine::RelationshipEngine;

let suggestions = RelationshipEngine::suggest_compensations(
    "Gain",
    0.5,  // old value
    0.8   // new value (+0.3 increase)
);

// Returns:
// [
//   ("bass", -0.1, "High gain can cause muddiness, reduce bass"),
//   ("mid", -0.05, "Scoop mids slightly for tighter sound")
// ]
```

**Implemented Relationships:**
- **High Gain** → Reduce bass (prevent muddiness)
- **High Gain** → Reduce mids (tighter sound)
- **High Treble** → Increase mids (balance)

---

### 5. State Diffing

#### `StateDiffer::diff`

Compare two state snapshots and generate a diff.

**Signature:**
```rust
pub fn diff(
    old_state: &[(i32, Vec<(i32, String, bool, Vec<(i32, String, f64, String)>)>)],
    new_state: &[(i32, Vec<(i32, String, bool, Vec<(i32, String, f64, String)>)>)],
) -> StateDiff
```

**Returns:**
```rust
pub struct StateDiff {
    pub changed_params: Vec<ParameterDiff>,
    pub new_fx: Vec<String>,
    pub removed_fx: Vec<String>,
    pub toggled_fx: Vec<(String, bool)>,
}
```

**Example:**
```rust
use ai_engine::StateDiffer;

let diff = StateDiffer::diff(&old_state, &new_state);

for change in &diff.changed_params {
    println!("{}: {} → {} (Δ {})",
        change.param_name,
        change.old_display,
        change.new_display,
        change.delta
    );
}
```

---

### 6. Transactions

#### `Transaction::new`

Create a new transaction for rollback support.

**Signature:**
```rust
pub fn new(actions: Vec<ActionPlan>) -> Self
```

**Example:**
```rust
use ai_engine::Transaction;

let transaction = Transaction::new(actions)
    .with_state(original_state);

// If something goes wrong, generate rollback actions
let rollback = transaction.rollback_actions();
// Execute rollback to restore original state
```

---

## Audio Analysis API

Low-level audio analysis functions.

### 1. Spectrum Analysis

#### `analyze_spectrum`

Perform FFT analysis on audio samples.

**Signature:**
```rust
pub fn analyze_spectrum(
    samples: &[f32],
    sample_rate: u32,
    config: &AnalysisConfig,
) -> FrequencySpectrum
```

**Parameters:**
```rust
pub struct AnalysisConfig {
    pub fft_size: usize,           // Default: 8192
    pub window_type: WindowType,   // Default: BlackmanHarris
    pub overlap: f32,              // Default: 0.75 (75%)
    pub frequency_bands: Vec<f32>, // Default: [31.5, 63, 125, ...]
}
```

**Window Types:**
- `Hann`: General purpose
- `Hamming`: General purpose
- `BlackmanHarris`: Best for audio analysis (default)
- `FlatTop`: Best for amplitude accuracy

**Example:**
```rust
use audio::analyzer::{analyze_spectrum, AnalysisConfig};

let config = AnalysisConfig::default();
let spectrum = analyze_spectrum(&samples, 48000, &config);

// spectrum.frequencies: Vec<f32> - Frequency bins in Hz
// spectrum.magnitudes: Vec<f32> - Magnitude in dB
```

---

### 2. EQ Profile Extraction

#### `extract_eq_profile`

Extract EQ profile from frequency spectrum.

**Signature:**
```rust
pub fn extract_eq_profile(
    spectrum: &FrequencySpectrum,
    config: &AnalysisConfig,
) -> EQProfile
```

**Returns:**
```rust
pub struct EQProfile {
    pub bands: Vec<FrequencyBand>,
    pub overall_loudness: f32,     // dB
    pub dynamic_range: f32,        // dB
    pub spectral_centroid: f32,    // Hz
    pub spectral_rolloff: f32,     // Hz
}

pub struct FrequencyBand {
    pub frequency: f32,      // Center frequency in Hz
    pub gain_db: f32,        // Gain in dB
    pub bandwidth: f32,      // Bandwidth in Hz
    pub confidence: f32,     // 0.0 - 1.0
}
```

**Example:**
```rust
use audio::profile::extract_eq_profile;

let profile = extract_eq_profile(&spectrum, &config);

for band in &profile.bands {
    println!("{} Hz: {:.2} dB (confidence: {:.2})",
        band.frequency, band.gain_db, band.confidence);
}
```

---

### 3. Profile Matching

#### `match_profiles`

Calculate EQ correction to match reference profile.

**Signature:**
```rust
pub fn match_profiles(
    reference: &EQProfile,
    input: &EQProfile,
    config: &MatchConfig,
) -> MatchResult
```

**Configuration:**
```rust
pub struct MatchConfig {
    pub intensity: f32,              // 0.0 - 1.0 (default: 0.7)
    pub max_correction: f32,         // Max ±dB per band (default: 6.0)
    pub smoothing_factor: f32,       // 0.0 - 1.0 (default: 0.5)
    pub use_psychoacoustic: bool,    // Apply Fletcher-Munson weighting (default: true)
    pub preserve_dynamics: bool,     // Don't compress dynamic range (default: true)
}
```

**Algorithm Steps:**
1. Normalize both profiles to their mean
2. Calculate raw differences
3. Apply psychoacoustic weighting (Fletcher-Munson)
4. Apply confidence-based attenuation
5. Smooth corrections across frequency bands
6. Apply intensity scaling
7. Limit to max_correction
8. Check for extreme corrections
9. Preserve dynamic range if enabled
10. Calculate quality score

**Example:**
```rust
use audio::matcher::{match_profiles, MatchConfig};

let config = MatchConfig {
    intensity: 0.8,
    max_correction: 8.0,
    smoothing_factor: 0.6,
    use_psychoacoustic: true,
    preserve_dynamics: true,
};

let result = match_profiles(&reference, &input, &config);

println!("Quality score: {:.2}", result.quality_score);
for warning in &result.warnings {
    println!("⚠️  {}", warning);
}
```

---

### 4. Audio Loading

#### `load_audio_file`

Load an audio file from disk.

**Signature:**
```rust
pub fn load_audio_file(path: &str) -> Result<AudioData, Box<dyn Error>>
```

**Returns:**
```rust
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}
```

**Supported Formats:** WAV, MP3, FLAC, OGG

---

#### `resample_audio`

Resample audio to a different sample rate.

**Signature:**
```rust
pub fn resample_audio(
    samples: &[f32],
    from_rate: u32,
    to_rate: u32,
) -> Result<Vec<f32>, Box<dyn Error>>
```

**Example:**
```rust
use audio::loader::{load_audio_file, resample_audio};

let audio = load_audio_file("input.wav")?;
let resampled = resample_audio(&audio.samples, audio.sample_rate, 48000)?;
```

---

## Type Definitions

### Core Types

#### `ChatMessage`
```rust
struct ChatMessage {
    role: String,        // "user" or "assistant"
    content: String,     // Message text
    timestamp: u64,      // Unix timestamp
}
```

---

#### `FxParamState`
```rust
struct FxParamState {
    index: i32,
    name: String,
    value: f64,           // Normalized 0.0 - 1.0
    display: String,      // e.g., "6.2 dB", "432 Hz"
    unit: String,         // e.g., "dB", "Hz", "%", "ms"
    format_hint: String,  // "decibel", "frequency", "percentage", "time", "raw"
}
```

---

#### `FxState`
```rust
struct FxState {
    index: i32,
    name: String,
    enabled: bool,
    params: Vec<FxParamState>,
}
```

---

#### `TrackSnapshot`
```rust
struct TrackSnapshot {
    index: i32,
    name: String,
    fx: Vec<FxState>,
}
```

---

#### `PlannedAction`
```rust
enum PlannedAction {
    SetParam {
        track: i32,
        fx_index: i32,
        param_index: i32,
        value: f64,           // 0.0 - 1.0
        reason: Option<String>,
    },
    ToggleFx {
        track: i32,
        fx_index: i32,
        enabled: bool,
        reason: Option<String>,
    },
    LoadPlugin {
        track: i32,
        plugin_name: String,
        position: Option<i32>,
        reason: Option<String>,
    },
    WebSearch {
        query: String,
        reason: Option<String>,
    },
    Noop {
        reason: Option<String>,
    },
}
```

---

#### `ChangeEntry`
```rust
struct ChangeEntry {
    plugin: String,
    parameter: String,
    old_value: String,    // Display value (e.g., "50%")
    new_value: String,    // Display value (e.g., "75%")
    reason: String,
}
```

---

#### `ChatResponse`
```rust
struct ChatResponse {
    summary: String,
    changes_table: Vec<ChangeEntry>,
}
```

---

### Audio Types

#### `EQProfile`
```rust
struct EQProfile {
    bands: Vec<FrequencyBand>,
    overall_loudness: f32,     // dB
    dynamic_range: f32,        // dB
    spectral_centroid: f32,    // Hz (center of mass of spectrum)
    spectral_rolloff: f32,     // Hz (85% of energy below this frequency)
}
```

---

#### `FrequencyBand`
```rust
struct FrequencyBand {
    frequency: f32,      // Center frequency in Hz
    gain_db: f32,        // Gain in dB
    bandwidth: f32,      // Bandwidth in Hz (for Q calculation)
    confidence: f32,     // 0.0 - 1.0 (measurement confidence)
}
```

---

#### `MatchResult`
```rust
struct MatchResult {
    correction_profile: EQProfile,
    reference_normalized: Vec<f32>,
    input_normalized: Vec<f32>,
    quality_score: f32,           // 0.0 - 1.0
    warnings: Vec<String>,
}
```

---

### AI Engine Types

#### `ParameterCategory`
```rust
enum ParameterCategory {
    Distortion,
    EQ,
    Dynamics,
    Modulation,
    Delay,
    Reverb,
    Filter,
    Volume,
    Toggle,
    Unknown,
}
```

---

#### `ActionPlan`
```rust
struct ActionPlan {
    track: i32,
    fx_index: i32,
    param_index: i32,
    value: f64,
    reason: String,
}
```

---

#### `ParameterDiff`
```rust
struct ParameterDiff {
    track: i32,
    fx_index: i32,
    param_index: i32,
    param_name: String,
    old_value: f64,
    new_value: f64,
    old_display: String,
    new_display: String,
    delta: f64,              // new_value - old_value
}
```

---

#### `StateDiff`
```rust
struct StateDiff {
    changed_params: Vec<ParameterDiff>,
    new_fx: Vec<String>,
    removed_fx: Vec<String>,
    toggled_fx: Vec<(String, bool)>,
}
```

---

#### `Transaction`
```rust
struct Transaction {
    id: String,                      // UUID
    actions: Vec<ActionPlan>,
    original_state: Vec<ParameterDiff>,
}
```

---

## Error Handling

### HTTP API Errors

All HTTP endpoints return JSON errors with appropriate status codes:

#### Status Codes
- `200`: Success
- `400`: Bad Request (invalid JSON, missing parameters)
- `404`: Not Found (track/FX/parameter not found)
- `500`: Internal Server Error (plugin load failure, etc.)

#### Error Response Format
```json
{
  "error": "Error message here"
}
```

#### Extended Error Response (Parameter Not Found)
```json
{
  "error": "Parameter not found",
  "searched": "invalid_param",
  "available_params": ["gain", "bass", "mid", "treble"]
}
```

---

### Tauri Command Errors

Tauri commands throw string errors that can be caught in try-catch blocks:

```typescript
try {
  await invoke('some_command', { param: value });
} catch (error) {
  console.error('Command failed:', error);
  // error is a string describing what went wrong
}
```

**Common Error Types:**
- `"AI provider is not configured"`: Call `configure_ai_provider` first
- `"Track not found"`: Invalid track index
- `"Failed to get FX params: 404"`: REAPER extension not running
- `"Load error: ..."`: Audio file not found or unsupported format
- `"Resample error: ..."`: Resampling failed

---

### Safety Validation Warnings

The AI Engine's `SafetyValidator` returns warnings for potentially problematic values:

```rust
let (clamped_value, warning) = SafetyValidator::validate_value("Gain", 1.5);

if let Some(warn) = warning {
    println!("⚠️  {}", warn);
    // "Value 1.5 above maximum 1.0, clamping"
}
```

**Warning Types:**
- Value below minimum (clamped)
- Value above maximum (clamped)
- Value exceeds recommended max (may cause clipping/distortion)

---

### Match Quality Warnings

EQ matching may produce warnings in the `MatchResult`:

```rust
pub struct MatchResult {
    // ...
    warnings: Vec<String>,
}
```

**Warning Types:**
- Correction limited (exceeded max_correction)
- Steep slope detected (>6 dB/octave)
- High total correction (>30 dB)

**Example:**
```
"125 Hz: Correction limited from 8.5 dB to 6.0 dB"
"Steep slope between 500 Hz and 1000 Hz (7.2 dB/octave)"
"High total correction: 35.2 dB. Consider lower intensity."
```

---

## Best Practices

### 1. REAPER Extension Connection

Always check connection before making API calls:

```typescript
const isConnected = await invoke('check_reaper_connection');
if (!isConnected) {
  console.error('REAPER extension is not running');
  return;
}
```

---

### 2. Parameter Value Ranges

Always use normalized values (0.0 - 1.0) when setting parameters:

```typescript
// ✅ Correct
await invoke('set_param', {
  track: 0,
  fx: 0,
  param: 'gain',
  value: 0.75  // 75%
});

// ❌ Incorrect
await invoke('set_param', {
  track: 0,
  fx: 0,
  param: 'gain',
  value: 75  // Will be clamped to 1.0!
});
```

---

### 3. AI Provider Configuration

Configure the AI provider before sending chat messages:

```typescript
// Configure first
await invoke('configure_ai_provider', {
  provider: 'gemini',
  model: 'gemini-1.5-pro',
  apiKey: API_KEY
});

// Then use
const response = await invoke('process_chat_message', {
  message: 'Make it heavier',
  track: 0
});
```

---

### 4. EQ Matching Intensity

Start with moderate intensity and adjust:

```typescript
// Subtle matching
const config = {
  intensity: 0.5,
  max_correction: 4.0,
  // ...
};

// Aggressive matching
const config = {
  intensity: 0.9,
  max_correction: 10.0,
  // ...
};
```

---

### 5. Error Handling

Always wrap API calls in try-catch:

```typescript
try {
  const result = await invoke('process_chat_message', {
    message: userInput,
    track: selectedTrack
  });
  const data = JSON.parse(result);
  // Process data...
} catch (error) {
  console.error('Failed to process message:', error);
  showErrorToUser(error);
}
```

---

### 6. Action Optimization

Use the AI Engine's optimizer before executing actions:

```rust
use ai_engine::ActionOptimizer;

// Detect conflicts
let conflicts = ActionOptimizer::detect_conflicts(&actions);
for conflict in &conflicts {
    eprintln!("⚠️  {}", conflict);
}

// Deduplicate
let deduplicated = ActionOptimizer::deduplicate(actions);

// Reorder for optimal execution
let optimized = ActionOptimizer::reorder(deduplicated);
```

---

## Examples

### Complete EQ Matching Workflow

```typescript
import { invoke } from '@tauri-apps/api/core';

async function matchEQ() {
  try {
    // 1. Load reference audio
    const refProfile = await invoke('load_reference_audio', {
      path: 'C:\\Audio\\reference.wav'
    });

    // 2. Load input audio
    const inputProfile = await invoke('load_input_audio', {
      path: 'C:\\Audio\\input.wav'
    });

    // 3. Calculate match
    const matchResult = await invoke('calculate_eq_match', {
      reference: refProfile,
      input: inputProfile,
      config: {
        intensity: 0.7,
        max_correction: 6.0,
        smoothing_factor: 0.5,
        use_psychoacoustic: true,
        preserve_dynamics: true
      }
    });

    console.log('Quality:', matchResult.quality_score);

    // 4. Show warnings
    for (const warning of matchResult.warnings) {
      console.warn(warning);
    }

    // 5. Export as REAPER preset
    const preset = await invoke('export_eq_settings', {
      result: matchResult,
      format: 'reaper'
    });

    // 6. Save to file
    await writeTextFile('eq_correction.fxchain', preset);

  } catch (error) {
    console.error('EQ matching failed:', error);
  }
}
```

---

### AI-Assisted Tone Adjustment

```typescript
import { invoke } from '@tauri-apps/api/core';

async function adjustTone() {
  try {
    // 1. Check REAPER connection
    const connected = await invoke('check_reaper_connection');
    if (!connected) {
      throw new Error('REAPER not connected');
    }

    // 2. Configure AI
    await invoke('configure_ai_provider', {
      provider: 'gemini',
      model: 'gemini-1.5-pro',
      apiKey: 'your-api-key'
    });

    // 3. Send message
    const responseJson = await invoke('process_chat_message', {
      message: 'Make this sound like a Metallica tone',
      track: 0
    });

    const response = JSON.parse(responseJson);

    // 4. Display results
    console.log('Summary:', response.summary);

    for (const change of response.changes_table) {
      console.log(`${change.plugin} :: ${change.parameter}`);
      console.log(`  ${change.old_value} → ${change.new_value}`);
      console.log(`  Reason: ${change.reason}`);
    }

  } catch (error) {
    console.error('Tone adjustment failed:', error);
  }
}
```

---

### Direct Parameter Control

```bash
#!/bin/bash

BASE_URL="http://127.0.0.1:8888"

# Add Neural DSP Gojira to track 0
curl -X POST $BASE_URL/fx/add \
  -H "Content-Type: application/json" \
  -d '{"track": 0, "plugin": "VST: Neural DSP Gojira"}'

# Set gain to 75%
curl -X POST $BASE_URL/fx/param \
  -H "Content-Type: application/json" \
  -d '{
    "track": 0,
    "fx": 0,
    "param": "gain",
    "value": 0.75
  }'

# Get all parameters
curl "$BASE_URL/fx/params?track=0&fx=0" | jq .

# Enable the FX
curl -X POST $BASE_URL/fx/toggle \
  -H "Content-Type: application/json" \
  -d '{"track": 0, "fx": 0, "enabled": true}'
```

---

## Versioning

API Version: **2.0**

**Version History:**
- **2.0**: AI Engine integration, comprehensive parameter metadata
- **1.0**: Initial HTTP API and Tauri commands

---

## Support

For issues and feature requests, please visit the [ToneForge GitHub repository](https://github.com/yourusername/toneforge).

---

**Document Version:** 1.0
**Last Updated:** December 2, 2025
**Generated from source code analysis**
