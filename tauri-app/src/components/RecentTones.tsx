import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RecentTone } from "../types";
import "./RecentTones.css";

interface RecentTonesProps {
  onSelectTone?: (query: string) => void;
}

export function RecentTones({ onSelectTone }: RecentTonesProps) {
  const [tones, setTones] = useState<RecentTone[]>([]);
  const [expanded, setExpanded] = useState(false);

  const fetchTones = async () => {
    try {
      const result = await invoke<string>("get_recent_tones");
      setTones(JSON.parse(result));
    } catch (error) {
      console.error("Failed to fetch recent tones:", error);
    }
  };

  useEffect(() => {
    fetchTones();
    const interval = setInterval(fetchTones, 10000);
    return () => clearInterval(interval);
  }, []);

  const handleClear = async () => {
    try {
      await invoke("clear_recent_tones");
      setTones([]);
    } catch (error) {
      console.error("Failed to clear recent tones:", error);
    }
  };

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return "Just now";
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return `${days}d ago`;
  };

  if (tones.length === 0) return null;

  const displayTones = expanded ? tones : tones.slice(0, 3);

  return (
    <div className="recent-tones">
      <div className="recent-header">
        <h4>Recent Tones</h4>
        <button className="ghost-btn" onClick={handleClear}>
          Clear
        </button>
      </div>
      <div className="recent-list">
        {displayTones.map((tone) => (
          <button
            key={tone.id}
            className="recent-item"
            onClick={() => onSelectTone?.(tone.query)}
            title={tone.summary}
          >
            <div className="recent-query">{tone.query}</div>
            <div className="recent-meta">
              <span className="recent-changes">{tone.changes_count} changes</span>
              <span className="recent-time">{formatTime(tone.timestamp)}</span>
            </div>
          </button>
        ))}
      </div>
      {tones.length > 3 && (
        <button
          className="expand-btn"
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? "Show less" : `Show ${tones.length - 3} more`}
        </button>
      )}
    </div>
  );
}
