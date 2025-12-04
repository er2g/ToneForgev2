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
