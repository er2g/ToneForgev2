import { useEffect, useCallback } from "react";

export interface ShortcutConfig {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
  meta?: boolean;
  action: () => void;
  description: string;
  enabled?: boolean;
}

export function useKeyboardShortcuts(shortcuts: ShortcutConfig[]) {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      for (const shortcut of shortcuts) {
        if (shortcut.enabled === false) continue;

        const keyMatch = e.key.toLowerCase() === shortcut.key.toLowerCase();
        const ctrlMatch = shortcut.ctrl ? e.ctrlKey || e.metaKey : !e.ctrlKey && !e.metaKey;
        const altMatch = shortcut.alt ? e.altKey : !e.altKey;
        const shiftMatch = shortcut.shift ? e.shiftKey : !e.shiftKey;

        if (keyMatch && ctrlMatch && altMatch && shiftMatch) {
          e.preventDefault();
          shortcut.action();
          break;
        }
      }
    },
    [shortcuts]
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);
}

// Common keyboard shortcuts
export const SHORTCUTS = {
  UNDO: { key: "z", ctrl: true, description: "Undo last action" },
  REDO: { key: "z", ctrl: true, shift: true, description: "Redo last action" },
  REDO_ALT: { key: "y", ctrl: true, description: "Redo last action" },
  SAVE_PRESET: { key: "s", ctrl: true, description: "Save preset" },
  SEARCH: { key: "f", ctrl: true, description: "Search FX" },
  NEW_MESSAGE: { key: "n", ctrl: true, description: "Focus chat input" },
  TOGGLE_THEME: { key: "t", ctrl: true, shift: true, description: "Toggle theme" },
  ESCAPE: { key: "Escape", description: "Cancel/close" },
};
