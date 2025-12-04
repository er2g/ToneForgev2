// Types for AI response with changes table
export interface ChangeEntry {
  plugin: string;
  parameter: string;
  old_value: string;
  new_value: string;
  reason: string;
}

export interface ChatResponse {
  summary: string;
  changes_table: ChangeEntry[];
  engine_report?: string;
  action_log?: string[];
}

// Undo/Redo types
export interface UndoState {
  can_undo: boolean;
  can_redo: boolean;
  undo_description: string | null;
  redo_description: string | null;
  undo_count: number;
  redo_count: number;
}

export interface UndoActionSummary {
  id: string;
  description: string;
  change_count: number;
  timestamp: number;
}

// Recent tones types
export interface RecentTone {
  id: string;
  query: string;
  summary: string;
  timestamp: number;
  track: number;
  changes_count: number;
}

// Secure storage types
export interface SecureConfig {
  api_key: string | null;
  provider: string | null;
  model: string | null;
  custom_instructions: string | null;
}

// Error response type
export interface ErrorResponse {
  code: string;
  message: string;
  suggestion: string;
  recoverable: boolean;
}

// Theme types
export type Theme = "dark" | "light";

// Keyboard shortcuts
export interface KeyboardShortcut {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  description: string;
  action: () => void;
}
