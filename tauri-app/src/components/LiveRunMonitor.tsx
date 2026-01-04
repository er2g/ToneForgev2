import { useEffect, useMemo, useRef, useState } from "react";
import "./LiveRunMonitor.css";

export type UiLogLevel = "debug" | "info" | "warn" | "error";

export interface UiLogEvent {
  request_id: string;
  timestamp_ms: number;
  stage: string;
  level: UiLogLevel | string;
  message: string;
  details?: unknown;
  step?: { current: number; total: number } | null;
}

function formatTime(ms: number) {
  const date = new Date(ms);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

function levelClass(level: string) {
  const l = level.toLowerCase();
  if (l === "error") return "error";
  if (l === "warn" || l === "warning") return "warn";
  if (l === "debug") return "debug";
  return "info";
}

function safeJson(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function LiveRunMonitor({
  title = "Live Monitor",
  logs,
  isRunning,
  onClear,
  autoScroll = true,
}: {
  title?: string;
  logs: UiLogEvent[];
  isRunning: boolean;
  onClear?: () => void;
  autoScroll?: boolean;
}) {
  const bottomRef = useRef<HTMLDivElement | null>(null);
  const [autoScrollEnabled, setAutoScrollEnabled] = useState(autoScroll);

  const lastStep = useMemo(() => {
    for (let i = logs.length - 1; i >= 0; i--) {
      const step = logs[i]?.step;
      if (step && typeof step.current === "number" && typeof step.total === "number") return step;
    }
    return null;
  }, [logs]);

  const progressPct = lastStep ? Math.round((lastStep.current / Math.max(1, lastStep.total)) * 100) : null;

  useEffect(() => {
    setAutoScrollEnabled(autoScroll);
  }, [autoScroll]);

  useEffect(() => {
    if (!autoScrollEnabled) return;
    bottomRef.current?.scrollIntoView({ block: "end" });
  }, [logs.length, autoScrollEnabled]);

  async function handleCopyJson() {
    try {
      await navigator.clipboard.writeText(safeJson(logs));
    } catch (e) {
      console.error("Failed to copy logs:", e);
    }
  }

  function handleDownloadJson() {
    try {
      const first = logs[0];
      const requestId = first?.request_id ?? "run";
      const blob = new Blob([safeJson(logs)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `toneforge-${requestId}-logs.json`;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Failed to download logs:", e);
    }
  }

  return (
    <section className="live-monitor">
      <div className="live-monitor-header">
        <div className="live-monitor-title">
          <span className={`live-dot ${isRunning ? "on" : "off"}`} />
          <span>{title}</span>
          <span className="live-monitor-meta">{logs.length} events</span>
        </div>
        <div className="live-monitor-actions">
          <button
            type="button"
            className="ghost-btn"
            onClick={() => setAutoScrollEnabled((v) => !v)}
          >
            Auto-scroll: {autoScrollEnabled ? "On" : "Off"}
          </button>
          <button
            type="button"
            className="ghost-btn"
            onClick={handleCopyJson}
            disabled={logs.length === 0}
          >
            Copy JSON
          </button>
          <button
            type="button"
            className="ghost-btn"
            onClick={handleDownloadJson}
            disabled={logs.length === 0}
          >
            Download
          </button>
          {onClear && (
            <button type="button" className="ghost-btn" onClick={onClear} disabled={logs.length === 0}>
              Clear
            </button>
          )}
        </div>
      </div>

      {lastStep && (
        <div className="live-monitor-progress">
          <div className="progress-bar">
            <div className="progress-fill" style={{ width: `${progressPct ?? 0}%` }} />
          </div>
          <div className="progress-text">
            {lastStep.current}/{lastStep.total} ({progressPct}%)
          </div>
        </div>
      )}

      <div className="live-monitor-body">
        {logs.length === 0 ? (
          <div className="live-monitor-empty">No live events yet.</div>
        ) : (
          <ul className="live-monitor-list">
            {logs.map((e, idx) => (
              <li key={`${e.request_id}-${e.timestamp_ms}-${idx}`} className={`live-item ${levelClass(e.level)}`}>
                <div className="live-item-main">
                  <span className="live-time">{formatTime(e.timestamp_ms)}</span>
                  <span className="live-stage">{e.stage}</span>
                  <span className="live-message">{e.message}</span>
                </div>
                {e.details !== undefined && (
                  <details className="live-details">
                    <summary>details</summary>
                    <pre>{safeJson(e.details)}</pre>
                  </details>
                )}
              </li>
            ))}
            <div ref={bottomRef} />
          </ul>
        )}
      </div>
    </section>
  );
}
