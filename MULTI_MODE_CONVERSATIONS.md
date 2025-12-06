# 🎸 ToneForge v2 - Multi-Mode AI Conversation System

## 🌟 Overview

ToneForge v2 now features **three independent AI modes**, each operating in **separate conversation rooms** - just like chatting with different experts!

```
┌─────────────────────────────────────────┐
│  🔍 Researcher  │  📋 Planner  │  ⚡ Act  │
├─────────────────────────────────────────┤
│ Conversation 1: "Metallica Tones"      │
│ Conversation 2: "Guitar EQ Analysis"   │
│ Conversation 3: "Apply Nirvana Tone"   │
└─────────────────────────────────────────┘
```

## 🎯 Three AI Modes

### 🔍 **Researcher Mode**
**Purpose**: Tone research and discussion
**REAPER**: Not connected
**Use Cases**:
- Research legendary guitar/bass tones
- Ask about equipment and techniques
- Discuss tone history and famous recordings
- Get tone recommendations
- Learn about artists' setups

**Example Conversations**:
- "Tell me about Metallica's Master of Puppets guitar tone"
- "What equipment did Chuck Schuldiner use on Death's Symbolic album?"
- "Compare Nirvana and Soundgarden grunge tones"
- "Recommend tones for modern metal"

**Features**:
- ✅ Searches tone encyclopedia
- ✅ AI-powered tone knowledge
- ✅ Equipment details
- ✅ Historical context
- ❌ NO REAPER modifications

---

### 📋 **Planner Mode**
**Purpose**: Analysis and planning
**REAPER**: Read-only access
**Use Cases**:
- Analyze current REAPER track state
- Get improvement suggestions
- Discuss potential tone modifications
- Plan plugin chain optimizations
- Identify EQ or gain issues

**Example Conversations**:
- "Analyze my current guitar tone"
- "What's wrong with my EQ?"
- "Suggest improvements for this track"
- "How can I make this tone tighter?"

**Features**:
- ✅ Reads REAPER state
- ✅ Plugin chain analysis
- ✅ Provides suggestions
- ✅ Educational explanations
- ❌ NO modifications (planning only!)

---

### ⚡ **Act Mode**
**Purpose**: Direct application
**REAPER**: Full read/write access
**Use Cases**:
- Apply tones directly to REAPER
- Use two-tier AI system
- Execute tone changes immediately
- Modify parameters with precision

**Example Conversations**:
- "Give me the Metallica Master of Puppets tone"
- "Apply Nirvana grunge tone"
- "Set up a death metal rhythm tone"

**Features**:
- ✅ **Tier 1**: Search encyclopedia or generate tone
- ✅ **Tier 2**: Map parameters to REAPER plugins
- ✅ Applies changes immediately
- ✅ Undo support
- ✅ Precision parameter control

---

## 🏗️ Architecture

### Conversation Management

```
ConversationManager
├─ Conversation 1 (Researcher)
│  ├─ Message 1: "Tell me about Metallica..."
│  ├─ Message 2: AI response with encyclopedia matches
│  └─ Message 3: "What about the Master of Puppets album?"
│
├─ Conversation 2 (Planner)
│  ├─ Message 1: "Analyze my tone"
│  ├─ Message 2: AI analysis with suggestions
│  └─ Message 3: "How do I fix the muddy bass?"
│
└─ Conversation 3 (Act)
   ├─ Message 1: "Give me Nirvana tone"
   └─ Message 2: AI applied 8 actions to REAPER ✓
```

### File Structure

```
tauri-app/src-tauri/src/
├─ conversation.rs       ← Conversation management
├─ researcher_mode.rs    ← 🔍 Researcher mode
├─ planner_mode.rs       ← 📋 Planner mode
├─ act_mode.rs           ← ⚡ Act mode (2-tier system)
├─ tone_encyclopedia.rs  ← Tone database
├─ ai_client.rs          ← Multi-provider AI
├─ tone_ai.rs            ← Tier 1: Tone AI
├─ parameter_ai.rs       ← Tier 2: Parameter AI
└─ lib.rs                ← Main app
```

---

## 📡 API Reference

### Create Conversation

```javascript
const conversationId = await invoke('create_conversation', {
  title: 'Metallica Tone Research',
  mode: 'researcher' // or 'planner', 'act'
});
```

### List Conversations

```javascript
const conversations = await invoke('list_conversations');
// Returns array of ConversationSummary objects
```

### Send Message

```javascript
const response = await invoke('send_message', {
  conversationId: 'conversation-uuid',
  message: 'Tell me about Metallica tones',
  trackIndex: 0 // optional, for Planner and Act modes
});
```

### Get Conversation

```javascript
const conversation = await invoke('get_conversation', {
  conversationId: 'conversation-uuid'
});
```

### Delete Conversation

```javascript
await invoke('delete_conversation', {
  conversationId: 'conversation-uuid'
});
```

### Clear Conversation Messages

```javascript
await invoke('clear_conversation', {
  conversationId: 'conversation-uuid'
});
```

---

## 🎨 UI Design Concept

### Sidebar (Conversation List)

```
┌──────────────────────────┐
│  📝 New Conversation     │
├──────────────────────────┤
│ 🔍 Metallica Research    │
│   Last: "Thanks for..."   │
│   5 minutes ago          │
├──────────────────────────┤
│ 📋 Guitar EQ Analysis    │
│   Last: "Try reducing..." │
│   1 hour ago             │
├──────────────────────────┤
│ ⚡ Apply Death Metal     │
│   Last: "Applied 8 act.."│
│   Yesterday              │
└──────────────────────────┘
```

### Chat View

```
┌─────────────────────────────────────────┐
│  🔍 Metallica Research              [⚙] │
├─────────────────────────────────────────┤
│                                         │
│  👤 You: Tell me about Metallica's      │
│          Master of Puppets tone         │
│                                         │
│  🤖 AI: Master of Puppets features an  │
│         aggressive thrash metal tone... │
│                                         │
│         📚 Found 2 matches in encyclo.  │
│         - Metallica - Battery (95%)     │
│         - Metallica - Lead (78%)        │
│                                         │
│  👤 You: What equipment did they use?   │
│                                         │
│  🤖 AI: For the rhythm tone on...      │
│                                         │
│ [Type your message...]         [Send]  │
└─────────────────────────────────────────┘
```

---

## 🎯 Usage Workflows

### Workflow 1: Research → Plan → Act

1. **🔍 Researcher Mode**: "Tell me about Metallica Master of Puppets tone"
   - AI searches encyclopedia
   - Returns tone details, equipment, parameters

2. **📋 Planner Mode**: "How would this tone work with my current setup?"
   - AI analyzes your REAPER state
   - Suggests modifications

3. **⚡ Act Mode**: "Apply the Metallica Battery tone"
   - AI applies tone to REAPER
   - Uses encyclopedia parameters
   - Maps to your plugins

### Workflow 2: Quick Application

1. **⚡ Act Mode**: "Give me a thrash metal tone"
   - AI searches encyclopedia
   - Finds best match or generates tone
   - Applies immediately to REAPER

### Workflow 3: Learning & Experimentation

1. **🔍 Researcher Mode**: "What's the difference between scooped and present mids?"
   - Learn about tone theory
   - Get examples from encyclopedia

2. **📋 Planner Mode**: "Does my tone have scooped mids?"
   - AI analyzes your current settings
   - Explains what you have

3. **⚡ Act Mode**: "Make my mids more present"
   - AI adjusts EQ parameters
   - Applies changes

---

## 💬 Message Metadata

Each message can include metadata:

```typescript
interface MessageMetadata {
  // For Act mode
  actions_count?: number;

  // For Researcher mode
  encyclopedia_matches?: number;

  // For all modes
  suggestions_count?: number;
  notes?: string[];
}
```

Example response:

```json
{
  "content": "I found 2 tones in the encyclopedia...",
  "metadata": {
    "encyclopedia_matches": 2,
    "suggestions_count": 3,
    "notes": [
      "Try scooping the mids",
      "Increase gain to 0.85",
      "Add a noise gate"
    ]
  }
}
```

---

## 🔧 Technical Details

### Conversation State

```rust
struct Conversation {
    id: String,
    title: String,
    mode: ConversationMode, // Researcher | Planner | Act
    created_at: u64,
    updated_at: u64,
    messages: Vec<Message>,
    active: bool,
    track_index: Option<i32>,
}
```

### Message Structure

```rust
struct Message {
    id: String,
    role: MessageRole, // User | Assistant | System
    content: String,
    timestamp: u64,
    metadata: Option<MessageMetadata>,
}
```

### Conversation Manager

- Stores all conversations in memory
- Tracks active/archived status
- Provides conversation history
- Supports filtering by mode

---

## 🎨 Benefits

### 1. **Separation of Concerns**
- Research without fear of changing REAPER
- Plan modifications before applying
- Execute with confidence

### 2. **Context Preservation**
- Each conversation maintains its own history
- AI remembers what you discussed
- Multi-turn conversations feel natural

### 3. **Flexibility**
- Use the right mode for the job
- Switch between modes easily
- Keep multiple projects separate

### 4. **Safety**
- Researcher mode: No REAPER access (safe exploration)
- Planner mode: Read-only (analyze without risk)
- Act mode: Full access (controlled execution)

---

## 📊 Example Scenarios

### Scenario 1: New to Metal Tones

```
🔍 Researcher Conversation: "Metal Tone Guide"
├─ "Tell me about death metal guitar tones"
├─ AI explains characteristics, references Chuck Schuldiner
├─ "What equipment do I need?"
├─ AI lists amps, guitars, pedals from encyclopedia
└─ "Show me specific examples"
    └─ AI provides Death - Symbolic, Metallica, etc.
```

### Scenario 2: Fixing Muddy Tone

```
📋 Planner Conversation: "Fix Muddy Guitar"
├─ "Why does my guitar sound muddy?"
├─ AI analyzes REAPER state: "Too much 200-400Hz"
├─ "How do I fix it?"
├─ AI suggests: "Cut 200Hz by -3dB, reduce bass to 0.4"
└─ (Switch to Act mode to apply)
```

### Scenario 3: Quick Setup

```
⚡ Act Conversation: "Nirvana Nevermind"
├─ "Give me Nirvana Smells Like Teen Spirit tone"
└─ AI: Encyclopedia match 92%, applied 6 actions ✓
```

---

## 🚀 Next Steps for Frontend

1. **Conversation List UI**
   - Sidebar with all conversations
   - Filter by mode (Researcher, Planner, Act)
   - New conversation button

2. **Chat Interface**
   - Message bubbles (user vs AI)
   - Metadata display (encyclopedia matches, suggestions)
   - Mode indicator

3. **Conversation Actions**
   - Rename conversation
   - Delete conversation
   - Clear messages
   - Archive/unarchive

4. **Smart Features**
   - Auto-title conversations based on first message
   - Search within conversations
   - Export conversation history

---

## 📝 Implementation Checklist

Backend:
- ✅ Conversation management system
- ✅ Researcher mode
- ✅ Planner mode
- ✅ Act mode
- ✅ Message metadata
- ✅ Tauri commands

Frontend (TODO):
- ⬜ Conversation list component
- ⬜ Chat interface component
- ⬜ Mode selector
- ⬜ Message display with metadata
- ⬜ New conversation dialog
- ⬜ Conversation settings

---

## 🎸 Philosophy

> **"The right tool for the job"**

- **Research** when you want to learn
- **Plan** when you want to analyze
- **Act** when you're ready to apply

Each mode is optimized for its purpose, giving you full control over how you interact with ToneForge's AI capabilities.

---

**Built with**: Rust, Tauri, React
**License**: Same as ToneForge project
**Status**: Backend complete, frontend pending
