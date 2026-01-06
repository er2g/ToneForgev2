# ToneForge v2 - Two-Tier AI Tone Generation System

## 🎸 Overview

ToneForge v2 has been completely rebuilt with a **two-tier AI architecture** that uses a **tone encyclopedia** for precision tone matching.

## 🏗️ Architecture

```
User Request
     │
     ├─► TIER 1: Tone AI
     │   ├─ Search Encyclopedia (thousands of album tones)
     │   ├─ If found: Use exact tone parameters
     │   └─ If not found: AI generates tone (+ optional supportive FX ideas)
     │
     ├─► REAPER Snapshot (current state)
     │
     └─► TIER 2: Parameter AI
         ├─ Maps tone parameters to REAPER plugins
         ├─ Fuzzy parameter matching (e.g., "gain" → "Drive", "Input", etc.)
         ├─ Precision value mapping
         ├─ Optional plugin loading + re-snapshot (multi-pass)
         └─ Apply to REAPER
```

## 🎯 Key Features

### 1. **Tone Encyclopedia**
- JSON database of guitar/bass tones from famous albums
- Format designed for easy contribution (you can add thousands of tones)
- Smart fuzzy search algorithm
- Example: `tone_encyclopedia.json` (currently 5 sample tones included)

### 2. **Multi-Provider AI Support**
- ✅ OpenAI (GPT-4, GPT-3.5)
- ✅ Anthropic Claude (Sonnet, Opus, Haiku)
- ✅ Google Gemini (Pro, Flash)
- ✅ xAI Grok

### 3. **Two-Tier Intelligence**

#### Tier 1: Tone AI (`tone_ai.rs`)
- Searches encyclopedia for matching tones
- Uses AI fallback when no match found
- Returns tone parameters (amp, EQ, effects, reverb, delay)

#### Tier 2: Parameter AI (`parameter_ai.rs`)
- Maps abstract tone parameters to specific REAPER plugins
- Intelligent parameter name matching
- Value normalization and safety validation
- Generates executable actions

## 📁 File Structure

```
tauri-app/src-tauri/src/
├── tone_encyclopedia.rs  ← Encyclopedia system
├── ai_client.rs          ← Multi-provider AI client
├── tone_ai.rs            ← Tier 1: Tone selection
├── parameter_ai.rs       ← Tier 2: Parameter mapping
└── lib.rs                ← Main app (simplified!)

tone_encyclopedia.json     ← Your tone database
```

## 🎵 Tone Encyclopedia Format

### Example Entry

```json
{
  "id": "metallica_master_of_puppets_battery",
  "artist": "Metallica",
  "album": "Master of Puppets",
  "song": "Battery",
  "year": 1986,
  "genre": "Thrash Metal",
  "instrument": "guitar",
  "description": "Aggressive thrash metal rhythm tone...",

  "equipment": {
    "guitar": "Gibson Explorer",
    "amp": "Mesa Boogie Mark IIC+",
    "cabinet": "Marshall 1960A 4x12",
    "pedals": []
  },

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
      "2kHz": 1.0
    },
    "effects": [
      {
        "effect_type": "noise_gate",
        "parameters": {
          "threshold": 0.3
        }
      }
    ]
  },

  "techniques": ["downpicking", "palm muting"],
  "tags": ["aggressive", "scooped", "tight"]
}
```

### Parameter Value Ranges

- **Amp/Effect parameters**: 0.0 to 1.0 (normalized)
- **EQ**: -12.0 dB to +12.0 dB
- **Reverb/Delay**: 0.0 to 1.0

## 🚀 Usage

### 1. Configure AI Provider

```javascript
await invoke('configure_ai_provider', {
  providerName: 'openai',  // or 'claude', 'gemini', 'grok'
  model: 'gpt-4',
  apiKey: 'your-api-key'
});
```

### 2. Process Tone Request

```javascript
const result = await invoke('process_tone_request', {
  message: 'Metallica Master of Puppets Battery tone',
  track: 0  // optional, defaults to 0
});

console.log(result);
// {
//   "tone_source": "Encyclopedia",
//   "tone_description": "Metallica - Master of Puppets - Battery | Aggressive...",
//   "confidence": 0.95,
//   "summary": "Applied 8 parameter changes to achieve the target tone",
//   "actions_count": 8,
//   "action_logs": [...]
// }
```

### 3. Load Custom Encyclopedia

```javascript
await invoke('load_encyclopedia', {
  path: '/path/to/your/encyclopedia.json'
});
```

### 4. Search Encyclopedia

```javascript
const results = await invoke('search_encyclopedia', {
  query: 'Nirvana grunge',
  limit: 10
});
```

## 🎛️ How It Works

### Example Request: "Give me the Metallica Master of Puppets Battery tone"

1. **Tier 1 (Tone AI)**:
   - Searches encyclopedia for "Metallica", "Master of Puppets", "Battery"
   - Finds exact match with 95% confidence
   - Returns tone parameters:
     ```json
     {
       "amp": {"gain": 0.85, "mid": 0.3, ...},
       "eq": {"800Hz": -4.0, "2kHz": 1.0, ...}
     }
     ```

2. **Tier 2 (Parameter AI)**:
   - Analyzes current REAPER plugins
   - Maps parameters:
     - `amp.gain: 0.85` → Plugin "Amp Sim" Parameter "Drive" = 0.85
     - `eq.800Hz: -4.0` → Plugin "ReaEQ" Band 3 = -4.0 dB
   - Generates actions:
     ```json
     [
       {
         "type": "set_param",
         "plugin_index": 0,
         "param_name": "Drive",
         "value": 0.85,
         "reason": "Setting amp gain for high-gain tone"
       }
     ]
     ```

3. **Apply to REAPER**:
   - Executes actions with undo support
   - Returns detailed logs

## 📝 Adding Tones to Encyclopedia

1. Open `tone_encyclopedia.json`
2. Add new entries following the format
3. Fill in all parameters precisely
4. Use tools like:
   - Guitar forums (Ultimate Guitar, Seymour Duncan Forums)
   - Equipment boards (Equipboard.com)
   - YouTube tutorials
   - Studio interviews

### Tips for Parameter Values

- **Gain**: 0.5 = clean, 0.7 = crunch, 0.85+ = high gain
- **Mids**: 0.3 = scooped (metal), 0.6+ = present (rock/blues)
- **EQ**: Subtract where muddy, boost where clarity needed
- **Be precise**: The AI will use these exact values

## 🔧 Technical Details

### Removed Old Code
- ❌ `ai_engine.rs` (complex optimization logic)
- ❌ `tone_researcher.rs` (web scraping)
- ❌ `xai_client.rs` (replaced with multi-provider client)
- ❌ `ai_engine_tests.rs`

### New Clean Code
- ✅ `tone_encyclopedia.rs` (560 lines)
- ✅ `ai_client.rs` (450 lines)
- ✅ `tone_ai.rs` (280 lines)
- ✅ `parameter_ai.rs` (380 lines)
- ✅ `lib.rs` (630 lines - **down from 2165!**)

### Code Reduction
- **Before**: ~2,600 lines of complex AI logic
- **After**: ~2,300 lines of clean, focused code
- **Complexity**: Significantly reduced

## 🎨 Benefits

1. **Precision**: Encyclopedia provides exact tone parameters
2. **Scalability**: Add thousands of tones easily
3. **Flexibility**: Multiple AI providers supported
4. **Maintainability**: Clean two-tier separation
5. **Transparency**: Clear pipeline (search → map → apply)

## 🚧 Next Steps

1. **Add more tones** to `tone_encyclopedia.json`
   - Start with your favorite albums
   - Include all equipment details
   - Be precise with parameters

2. **Test with your setup**
   - Try different AI providers
   - Test encyclopedia search
   - Validate parameter mapping

3. **Contribute tones**
   - Build a comprehensive database
   - Share with the community

## 📊 Sample Tones Included

1. **Metallica - Master of Puppets** (Battery) - Thrash rhythm
2. **Metallica - Master of Puppets** (Lead) - Solo tone
3. **Nirvana - Nevermind** - Grunge
4. **Pink Floyd - The Wall** (Comfortably Numb) - Legendary lead
5. **Death - Symbolic** - Technical death metal

## 🎯 Philosophy

> "The best tone is the one you can **reproduce exactly**."

By building a comprehensive tone encyclopedia with precise parameters, ToneForge v2 empowers you to:
- Instantly recall legendary album tones
- Learn from professional settings
- Build your own tone library

---

**Built with**: Rust, Tauri, React, and multiple AI providers
**License**: Same as ToneForge project
**Contributions**: Encyclopedia contributions welcome!
