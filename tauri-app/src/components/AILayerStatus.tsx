import { useEffect, useState } from "react";
import "./AILayerStatus.css";

export type AILayerPhase =
  | "detecting"
  | "researching"
  | "implementing"
  | "optimizing"
  | "applying"
  | "done";

interface AILayerStatusProps {
  phase: AILayerPhase;
  message?: string;
}

const PHASE_CONFIG: Record<
  AILayerPhase,
  { icon: string; label: string; color: string; description: string }
> = {
  detecting: {
    icon: "🔍",
    label: "Starting",
    color: "#3b82f6",
    description: "Preparing the pipeline...",
  },
  researching: {
    icon: "📚",
    label: "Layer 1: Tone AI",
    color: "#8b5cf6",
    description: "Searching the encyclopedia / generating parameters...",
  },
  implementing: {
    icon: "🎛️",
    label: "Layer 2: Mapping",
    color: "#f59e0b",
    description: "Sanitizing and mapping parameters to your chain...",
  },
  optimizing: {
    icon: "⚡",
    label: "Validating",
    color: "#10b981",
    description: "Validating and preparing actions...",
  },
  applying: {
    icon: "🎸",
    label: "Applying + Verify",
    color: "#ff6b35",
    description: "Applying actions to REAPER and verifying...",
  },
  done: {
    icon: "✅",
    label: "Done",
    color: "#22c55e",
    description: "Completed successfully.",
  },
};

export function AILayerStatus({ phase, message }: AILayerStatusProps) {
  const [progress, setProgress] = useState(0);
  const [dots, setDots] = useState("");
  const config = PHASE_CONFIG[phase];

  // Animated dots for loading effect
  useEffect(() => {
    const interval = setInterval(() => {
      setDots((prev) => (prev.length >= 3 ? "" : prev + "."));
    }, 500);
    return () => clearInterval(interval);
  }, []);

  // Progress animation
  useEffect(() => {
    const targetProgress = phase === "done" ? 100 :
      phase === "applying" ? 90 :
      phase === "optimizing" ? 75 :
      phase === "implementing" ? 60 :
      phase === "researching" ? 40 :
      phase === "detecting" ? 20 : 0;

    const interval = setInterval(() => {
      setProgress((prev) => {
        if (prev >= targetProgress) return prev;
        const increment = Math.max(1, (targetProgress - prev) / 10);
        return Math.min(targetProgress, prev + increment);
      });
    }, 100);

    return () => clearInterval(interval);
  }, [phase]);

  return (
    <div className="ai-layer-status">
      <div className="status-header">
        <div className="status-icon" style={{ color: config.color }}>
          <span className="icon-pulse">{config.icon}</span>
        </div>
        <div className="status-info">
          <div className="status-label">{config.label}</div>
          <div className="status-description">
            {message || config.description}
            {phase !== "done" && <span className="dots">{dots}</span>}
          </div>
        </div>
      </div>

      <div className="progress-container">
        <div className="progress-bar">
          <div
            className="progress-fill"
            style={{
              width: `${progress}%`,
              backgroundColor: config.color,
            }}
          />
        </div>
        <div className="progress-text">{Math.round(progress)}%</div>
      </div>

      {/* Layer indicators */}
      <div className="layer-indicators">
        <div
          className={`layer-indicator ${
            ["detecting", "researching"].includes(phase) ? "active" : "completed"
          }`}
        >
          <div className="indicator-dot" style={{ borderColor: PHASE_CONFIG.researching.color }} />
          <span>Layer 1: Research</span>
        </div>
        <div className="layer-connector" />
        <div
          className={`layer-indicator ${
            ["implementing", "optimizing", "applying"].includes(phase)
              ? "active"
              : phase === "done"
              ? "completed"
              : "pending"
          }`}
        >
          <div className="indicator-dot" style={{ borderColor: PHASE_CONFIG.implementing.color }} />
          <span>Layer 2: Implementation</span>
        </div>
      </div>
    </div>
  );
}
