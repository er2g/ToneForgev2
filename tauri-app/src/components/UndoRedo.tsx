import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { UndoState } from "../types";
import "./UndoRedo.css";

interface UndoRedoProps {
  onAction?: (action: "undo" | "redo", description: string) => void;
}

export function UndoRedo({ onAction }: UndoRedoProps) {
  const [state, setState] = useState<UndoState>({
    can_undo: false,
    can_redo: false,
    undo_description: null,
    redo_description: null,
    undo_count: 0,
    redo_count: 0,
  });
  const [loading, setLoading] = useState(false);

  const fetchState = useCallback(async () => {
    try {
      const result = await invoke<string>("get_undo_state");
      setState(JSON.parse(result));
    } catch (error) {
      console.error("Failed to fetch undo state:", error);
    }
  }, []);

  useEffect(() => {
    fetchState();
    const interval = setInterval(fetchState, 2000);
    return () => clearInterval(interval);
  }, [fetchState]);

  const handleUndo = async () => {
    if (!state.can_undo || loading) return;
    setLoading(true);
    try {
      const result = await invoke<string>("perform_undo");
      onAction?.("undo", result);
      await fetchState();
    } catch (error) {
      console.error("Undo failed:", error);
    } finally {
      setLoading(false);
    }
  };

  const handleRedo = async () => {
    if (!state.can_redo || loading) return;
    setLoading(true);
    try {
      const result = await invoke<string>("perform_redo");
      onAction?.("redo", result);
      await fetchState();
    } catch (error) {
      console.error("Redo failed:", error);
    } finally {
      setLoading(false);
    }
  };

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "z") {
        if (e.shiftKey) {
          e.preventDefault();
          handleRedo();
        } else {
          e.preventDefault();
          handleUndo();
        }
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "y") {
        e.preventDefault();
        handleRedo();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [state.can_undo, state.can_redo, loading]);

  return (
    <div className="undo-redo-container">
      <button
        className={`undo-btn ${!state.can_undo || loading ? "disabled" : ""}`}
        onClick={handleUndo}
        disabled={!state.can_undo || loading}
        title={state.undo_description ? `Undo: ${state.undo_description}` : "Nothing to undo (Ctrl+Z)"}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M12.5 8c-2.65 0-5.05.99-6.9 2.6L2 7v9h9l-3.62-3.62c1.39-1.16 3.16-1.88 5.12-1.88 3.54 0 6.55 2.31 7.6 5.5l2.37-.78C21.08 11.03 17.15 8 12.5 8z" />
        </svg>
        {state.undo_count > 0 && <span className="count">{state.undo_count}</span>}
      </button>
      <button
        className={`redo-btn ${!state.can_redo || loading ? "disabled" : ""}`}
        onClick={handleRedo}
        disabled={!state.can_redo || loading}
        title={state.redo_description ? `Redo: ${state.redo_description}` : "Nothing to redo (Ctrl+Shift+Z)"}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M18.4 10.6C16.55 8.99 14.15 8 11.5 8c-4.65 0-8.58 3.03-9.96 7.22L3.9 16c1.05-3.19 4.05-5.5 7.6-5.5 1.95 0 3.73.72 5.12 1.88L13 16h9V7l-3.6 3.6z" />
        </svg>
        {state.redo_count > 0 && <span className="count">{state.redo_count}</span>}
      </button>
    </div>
  );
}
