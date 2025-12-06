//! Conversation Management System
//!
//! This module manages multiple AI conversations with different modes:
//! - Researcher: Tone research and discussion (no REAPER connection)
//! - Planner: Analysis and suggestions (read-only REAPER)
//! - Act: Direct application (full 2-tier system)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Conversation mode type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversationMode {
    Researcher, // Tone research and discussion
    Planner,    // Analysis and suggestions
    Act,        // Direct application
}

impl ConversationMode {
    pub fn name(&self) -> &str {
        match self {
            ConversationMode::Researcher => "Researcher",
            ConversationMode::Planner => "Planner",
            ConversationMode::Act => "Act",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ConversationMode::Researcher => "Research tones and discuss music history",
            ConversationMode::Planner => "Analyze REAPER state and suggest improvements",
            ConversationMode::Act => "Apply tones directly to REAPER",
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            ConversationMode::Researcher => "🔍",
            ConversationMode::Planner => "📋",
            ConversationMode::Act => "⚡",
        }
    }
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// For Act mode: number of actions applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions_count: Option<usize>,

    /// For Researcher mode: encyclopedia matches found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encyclopedia_matches: Option<usize>,

    /// For Planner mode: suggestions count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions_count: Option<usize>,

    /// Any warnings or notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

/// A conversation (chat room)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub mode: ConversationMode,
    pub created_at: u64,
    pub updated_at: u64,
    pub messages: Vec<Message>,
    pub active: bool,

    /// Optional track index for Planner and Act modes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_index: Option<i32>,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(title: String, mode: ConversationMode) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            mode,
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
            active: true,
            track_index: None,
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, role: MessageRole, content: String, metadata: Option<MessageMetadata>) {
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content,
            timestamp: current_timestamp(),
            metadata,
        };

        self.messages.push(message);
        self.updated_at = current_timestamp();
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get last message
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Get messages for AI context (last N messages)
    pub fn get_context_messages(&self, limit: usize) -> Vec<&Message> {
        let start = if self.messages.len() > limit {
            self.messages.len() - limit
        } else {
            0
        };
        self.messages[start..].iter().collect()
    }

    /// Clear all messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.updated_at = current_timestamp();
    }

    /// Archive conversation
    pub fn archive(&mut self) {
        self.active = false;
        self.updated_at = current_timestamp();
    }

    /// Restore conversation
    pub fn restore(&mut self) {
        self.active = true;
        self.updated_at = current_timestamp();
    }
}

/// Conversation manager
pub struct ConversationManager {
    conversations: HashMap<String, Conversation>,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
        }
    }

    /// Create a new conversation
    pub fn create_conversation(&mut self, title: String, mode: ConversationMode) -> String {
        let conversation = Conversation::new(title, mode);
        let id = conversation.id.clone();
        self.conversations.insert(id.clone(), conversation);
        id
    }

    /// Get conversation by ID
    pub fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.conversations.get(id)
    }

    /// Get mutable conversation by ID
    pub fn get_conversation_mut(&mut self, id: &str) -> Option<&mut Conversation> {
        self.conversations.get_mut(id)
    }

    /// List all conversations
    pub fn list_conversations(&self) -> Vec<&Conversation> {
        let mut convs: Vec<&Conversation> = self.conversations.values().collect();
        convs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        convs
    }

    /// List active conversations
    pub fn list_active_conversations(&self) -> Vec<&Conversation> {
        let mut convs: Vec<&Conversation> = self
            .conversations
            .values()
            .filter(|c| c.active)
            .collect();
        convs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        convs
    }

    /// List conversations by mode
    pub fn list_conversations_by_mode(&self, mode: ConversationMode) -> Vec<&Conversation> {
        let mut convs: Vec<&Conversation> = self
            .conversations
            .values()
            .filter(|c| c.mode == mode)
            .collect();
        convs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        convs
    }

    /// Delete conversation
    pub fn delete_conversation(&mut self, id: &str) -> bool {
        self.conversations.remove(id).is_some()
    }

    /// Add message to conversation
    pub fn add_message(
        &mut self,
        conversation_id: &str,
        role: MessageRole,
        content: String,
        metadata: Option<MessageMetadata>,
    ) -> Result<(), String> {
        let conversation = self
            .conversations
            .get_mut(conversation_id)
            .ok_or_else(|| "Conversation not found".to_string())?;

        conversation.add_message(role, content, metadata);
        Ok(())
    }

    /// Clear conversation messages
    pub fn clear_conversation(&mut self, id: &str) -> Result<(), String> {
        let conversation = self
            .conversations
            .get_mut(id)
            .ok_or_else(|| "Conversation not found".to_string())?;

        conversation.clear_messages();
        Ok(())
    }

    /// Archive conversation
    pub fn archive_conversation(&mut self, id: &str) -> Result<(), String> {
        let conversation = self
            .conversations
            .get_mut(id)
            .ok_or_else(|| "Conversation not found".to_string())?;

        conversation.archive();
        Ok(())
    }

    /// Get conversation count
    pub fn count(&self) -> usize {
        self.conversations.len()
    }

    /// Get active conversation count
    pub fn active_count(&self) -> usize {
        self.conversations.values().filter(|c| c.active).count()
    }
}

impl Default for ConversationManager {
    fn default() -> Self {
        Self::new()
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Conversation summary for listing
#[derive(Debug, Clone, Serialize)]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub mode: ConversationMode,
    pub message_count: usize,
    pub last_message_preview: Option<String>,
    pub updated_at: u64,
    pub active: bool,
}

impl From<&Conversation> for ConversationSummary {
    fn from(conv: &Conversation) -> Self {
        let last_message_preview = conv.last_message().map(|msg| {
            let preview_len = 50;
            if msg.content.len() > preview_len {
                format!("{}...", &msg.content[..preview_len])
            } else {
                msg.content.clone()
            }
        });

        ConversationSummary {
            id: conv.id.clone(),
            title: conv.title.clone(),
            mode: conv.mode,
            message_count: conv.message_count(),
            last_message_preview,
            updated_at: conv.updated_at,
            active: conv.active,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let conv = Conversation::new("Test Conversation".to_string(), ConversationMode::Researcher);
        assert_eq!(conv.mode, ConversationMode::Researcher);
        assert_eq!(conv.messages.len(), 0);
        assert!(conv.active);
    }

    #[test]
    fn test_conversation_manager() {
        let mut manager = ConversationManager::new();

        let id = manager.create_conversation("Research Metallica".to_string(), ConversationMode::Researcher);
        assert_eq!(manager.count(), 1);

        manager.add_message(&id, MessageRole::User, "Tell me about Metallica tones".to_string(), None).unwrap();

        let conv = manager.get_conversation(&id).unwrap();
        assert_eq!(conv.message_count(), 1);
    }

    #[test]
    fn test_conversation_modes() {
        assert_eq!(ConversationMode::Researcher.name(), "Researcher");
        assert_eq!(ConversationMode::Planner.icon(), "📋");
        assert_eq!(ConversationMode::Act.description(), "Apply tones directly to REAPER");
    }
}
