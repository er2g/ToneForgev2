import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EqMatchView } from "./eq-match/EqMatchView";
import { ChangesTable } from "./components/ChangesTable";
import { AILayerStatus, AILayerPhase } from "./components/AILayerStatus";
import { Toast, ToastMessage } from "./components/Toast";
import { TypingIndicator } from "./components/TypingIndicator";
import { ChatResponse, ChangeEntry } from "./types";
import "./App.css";

const PROVIDERS = [
  { key: "gemini", label: "Google Gemini" },
] as const;

type ProviderKey = (typeof PROVIDERS)[number]["key"];

const MODEL_PRESETS: Record<ProviderKey, string[]> = {
  gemini: ["gemini-2.0-flash", "gemini-2.0-pro-exp", "gemini-1.5-pro"],
};

interface Message {
  role: "user" | "assistant";
  content: string;
  changes_table?: ChangeEntry[];
  timestamp?: number;
}

interface TrackFX {
  index: number;
  name: string;
  enabled: boolean;
}

interface TrackInfo {
  index: number;
  name: string;
  fx_count: number;
  fx_list: TrackFX[];
}

interface TrackResponse {
  track_count: number;
  tracks: TrackInfo[];
}

const HISTORY_STORAGE_KEY = "toneforge_history";

function App() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [apiKeySet, setApiKeySet] = useState(false);
  const [reaperConnected, setReaperConnected] = useState(false);
  const [loading, setLoading] = useState(false);
  const [aiPhase, setAiPhase] = useState<AILayerPhase | null>(null);
  const [aiMessage, setAiMessage] = useState<string>("");
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const [tracks, setTracks] = useState<TrackInfo[]>([]);
  const [selectedTrack, setSelectedTrack] = useState(0);
  const [provider, setProvider] = useState<ProviderKey>("gemini");
  const [model, setModel] = useState(MODEL_PRESETS.gemini[0]);
  const [activeView, setActiveView] = useState<"assistant" | "eq">("assistant");

  const currentTrack =
    tracks.find((track) => track.index === selectedTrack) ?? tracks[0];
  const currentTrackFx = currentTrack?.fx_list ?? [];
  const activeFxCount = currentTrackFx.filter((fx) => fx.enabled).length;
  const readyForChat = apiKeySet && reaperConnected;

  // Toast helper
  const addToast = (type: ToastMessage["type"], message: string, duration?: number) => {
    const id = `toast-${Date.now()}-${Math.random()}`;
    setToasts((prev) => [...prev, { id, type, message, duration }]);
  };

  const dismissToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  useEffect(() => {
    const cached = localStorage.getItem(HISTORY_STORAGE_KEY);
    if (cached) {
      try {
        setMessages(JSON.parse(cached));
      } catch {
        // ignore parse error
      }
    }
  }, []);

  useEffect(() => {
    if (messages.length) {
      localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(messages));
    } else {
      localStorage.removeItem(HISTORY_STORAGE_KEY);
    }
  }, [messages]);

  useEffect(() => {
    checkReaperConnection();
    const interval = setInterval(checkReaperConnection, 5000);
    return () => clearInterval(interval);
  }, []);

  async function checkReaperConnection() {
    try {
      const connected = await invoke<boolean>("check_reaper_connection");
      setReaperConnected(connected);
      if (connected) {
        await loadTrackOverview();
      }
    } catch {
      setReaperConnected(false);
    }
  }

  async function loadTrackOverview() {
    try {
      const overview = await invoke<string>("get_track_overview");
      const parsed: TrackResponse = JSON.parse(overview);
      const detectedTracks = Array.isArray(parsed.tracks) ? parsed.tracks : [];
      setTracks(detectedTracks);
      if (detectedTracks.length === 0) {
        setSelectedTrack(0);
        return;
      }
      const hasSelected = detectedTracks.some(
        (track) => track.index === selectedTrack
      );
      if (!hasSelected) {
        setSelectedTrack(detectedTracks[0].index);
      }
    } catch (error) {
      console.error("Failed to load track overview:", error);
    }
  }

  async function fetchChatHistory(): Promise<Message[]> {
    try {
      const historyJson = await invoke<string>("get_chat_history");
      const history = JSON.parse(historyJson) as Message[];
      return history;
    } catch (error) {
      console.error("Failed to load chat history:", error);
      return [];
    }
  }

  async function handleConfigureAssistant() {
    if (!apiKey.trim()) {
      addToast("warning", "Please enter a valid API key");
      return;
    }
    if (!model.trim()) {
      addToast("warning", "Please select or enter a model name");
      return;
    }

    try {
      await invoke<string>("configure_ai_provider", {
        provider,
        model,
        apiKey,
      });
      const history = await fetchChatHistory();
      if (history.length > 0) {
        setMessages(history);
      } else {
        setMessages([
          {
            role: "assistant",
            content:
              "Tone assistant ready. Pick a channel and tell me what sound you're after.",
            timestamp: Date.now(),
          },
        ]);
      }
      setApiKeySet(true);
      addToast("success", "AI Assistant configured successfully!");
    } catch (error) {
      addToast("error", "Failed to configure AI provider: " + error);
    }
  }

  async function handleSendMessage() {
    if (!input.trim() || !readyForChat) return;

    const userMessage: Message = {
      role: "user",
      content: input,
      timestamp: Date.now(),
    };
    setMessages((prev) => [...prev, userMessage]);
    const payload = input;
    const userInput = input;
    setInput("");
    setLoading(true);

    // AI Phase simulation - in real implementation, backend would send progress updates
    const simulatePhases = async () => {
      // Check if message contains tone request keywords
      const toneKeywords = ["tone", "sound", "tonu", "ses"];
      const hasToneRequest = toneKeywords.some(keyword =>
        userInput.toLowerCase().includes(keyword)
      );

      if (hasToneRequest) {
        // Phase 1: Detecting
        setAiPhase("detecting");
        setAiMessage("Analyzing your tone request...");
        await new Promise(resolve => setTimeout(resolve, 800));

        // Phase 2: Researching
        setAiPhase("researching");
        setAiMessage("Searching internet for tone details...");
        await new Promise(resolve => setTimeout(resolve, 2000));

        // Phase 3: Implementing
        setAiPhase("implementing");
        setAiMessage("Matching plugins to research results...");
        await new Promise(resolve => setTimeout(resolve, 1500));
      } else {
        // Simple query - skip research phase
        setAiPhase("implementing");
        setAiMessage("Processing your request...");
        await new Promise(resolve => setTimeout(resolve, 1000));
      }

      // Phase 4: Optimizing
      setAiPhase("optimizing");
      setAiMessage("AI Engine optimizations...");
      await new Promise(resolve => setTimeout(resolve, 1000));

      // Phase 5: Applying
      setAiPhase("applying");
      setAiMessage("Setting parameters in REAPER...");
    };

    try {
      // Start phase simulation
      const phasePromise = simulatePhases();

      // Actual API call
      const responseString = await invoke<string>("process_chat_message", {
        message: payload,
        track: selectedTrack,
      });

      // Wait for phase animation to complete
      await phasePromise;

      const response: ChatResponse = JSON.parse(responseString);

      // Done phase
      setAiPhase("done");
      setAiMessage("Tone created successfully!");
      await new Promise(resolve => setTimeout(resolve, 1000));

      const assistantMessage: Message = {
        role: "assistant",
        content: response.summary,
        changes_table: response.changes_table,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      await loadTrackOverview();

      // Success toast
      if (response.changes_table && response.changes_table.length > 0) {
        addToast("success", `Applied ${response.changes_table.length} changes successfully!`);
      }
    } catch (error) {
      const errorMessage: Message = {
        role: "assistant",
        content: "Error: " + error,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, errorMessage]);
      addToast("error", "Failed to process message: " + error);
    } finally {
      setLoading(false);
      setAiPhase(null);
      setAiMessage("");
    }
  }

  async function handleSavePreset() {
    const presetName = prompt("Preset name:");
    if (!presetName) return;

    try {
      const path = await invoke<string>("save_preset", { name: presetName });
      addToast("success", `Preset saved: ${presetName}`, 6000);
    } catch (error) {
      addToast("error", "Failed to save preset: " + error);
    }
  }

  async function handleToggleFx(
    trackIndex: number,
    fxIndex: number,
    enabled: boolean
  ) {
    try {
      await invoke<boolean>("set_fx_enabled", {
        track: trackIndex,
        fx: fxIndex,
        enabled: !enabled,
      });
      await loadTrackOverview();
      addToast("info", `FX ${enabled ? "disabled" : "enabled"}`, 2000);
    } catch (error) {
      addToast("error", "Failed to toggle FX: " + error);
    }
  }

  return (
    <div className="app-container">
      <Toast toasts={toasts} onDismiss={dismissToast} />
      <header className="app-header">
        <div className="header-content">
          <h1>🎸 ToneForge</h1>
          <div className="header-actions">
            <div
              className={`status-pill ${reaperConnected ? "online" : "offline"}`}
            >
              {reaperConnected ? "REAPER Online" : "REAPER Offline"}
            </div>
            <div className="view-switcher">
              <button
                type="button"
                className={activeView === "assistant" ? "active" : ""}
                onClick={() => setActiveView("assistant")}
              >
                Assistant
              </button>
              <button
                type="button"
                className={activeView === "eq" ? "active" : ""}
                onClick={() => setActiveView("eq")}
              >
                EQ Match
              </button>
            </div>
            <button
              className="primary-btn"
              onClick={handleSavePreset}
              disabled={!reaperConnected}
            >
              Save Preset
            </button>
          </div>
        </div>
      </header>

      <main className="app-main">
        {activeView === "assistant" ? (
          <>
            <aside className="sidebar">
              {apiKeySet ? (
                <>
                  <div className="sidebar-section">
                    <div className="section-header">
                      <h3>Channels</h3>
                      <button className="ghost-btn" onClick={loadTrackOverview}>
                        Refresh
                      </button>
                    </div>
                    {tracks.length === 0 ? (
                      <p className="empty-state">No channels detected</p>
                    ) : (
                      <div className="channel-grid">
                        {tracks.map((track) => (
                          <div
                            key={track.index}
                            className={`channel-card ${
                              selectedTrack === track.index ? "selected" : ""
                            }`}
                            onClick={() => setSelectedTrack(track.index)}
                          >
                            <div className="channel-title">
                              <span>Channel {track.index + 1}</span>
                              <span
                                className={`channel-dot ${
                                  track.fx_list.some((fx) => fx.enabled)
                                    ? "active"
                                    : "inactive"
                                }`}
                              />
                            </div>
                            <div className="channel-name">{track.name}</div>
                            <div className="channel-meta">
                              {track.fx_list.filter((fx) => fx.enabled).length} active / {" "}
                              {track.fx_count} FX
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  <div className="sidebar-section">
                    <div className="section-header">
                      <h3>FX Chain</h3>
                      {currentTrack && (
                        <span className="section-meta">
                          {currentTrack.name} · Channel {currentTrack.index + 1}
                        </span>
                      )}
                    </div>
                    {currentTrack && currentTrackFx.length > 0 ? (
                      <ul className="fx-list">
                        {currentTrackFx.map((fx) => (
                          <li
                            key={fx.index}
                            className={`fx-item ${fx.enabled ? "enabled" : "disabled"}`}
                          >
                            <div className="fx-details">
                              <span className="fx-name">
                                {fx.index + 1}. {fx.name}
                              </span>
                              <span
                                className={`fx-status ${fx.enabled ? "on" : "off"}`}
                              >
                                {fx.enabled ? "Active" : "Bypassed"}
                              </span>
                            </div>
                            <button
                              className="ghost-btn"
                              onClick={() =>
                                currentTrack &&
                                handleToggleFx(currentTrack.index, fx.index, fx.enabled)
                              }
                            >
                              {fx.enabled ? "Disable" : "Enable"}
                            </button>
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <p className="empty-state">No plugins loaded</p>
                    )}
                    <div className="fx-summary">
                      {activeFxCount} active FX on this channel
                    </div>
                  </div>
                </>
              ) : (
                <div className="setup-panel">
                  <h2>Connect ToneForge</h2>
                  <p>Enter your API key to unlock the assistant.</p>

                  <div className="status">
                    <div
                      className={`status-indicator ${
                        reaperConnected ? "connected" : "disconnected"
                      }`}
                    >
                      {reaperConnected ? "ON" : "OFF"}
                    </div>
                    <span>
                      REAPER {reaperConnected ? "Connected" : "Disconnected"}
                    </span>
                  </div>

                  <div className="api-config">
                    <label htmlFor="provider">API Provider</label>
                    <select
                      id="provider"
                      value={provider}
                      onChange={(e) => {
                        const value = e.target.value as ProviderKey;
                        setProvider(value);
                        setModel(MODEL_PRESETS[value][0]);
                      }}
                    >
                      {PROVIDERS.map((item) => (
                        <option key={item.key} value={item.key}>
                          {item.label}
                        </option>
                      ))}
                    </select>
                  </div>

                  <div className="api-config">
                    <label htmlFor="model">Model</label>
                    <input
                      id="model"
                      list="model-options"
                      placeholder="gemini-2.0-flash"
                      value={model}
                      onChange={(e) => setModel(e.target.value)}
                    />
                    <datalist id="model-options">
                      {MODEL_PRESETS[provider].map((entry) => (
                        <option key={entry} value={entry} />
                      ))}
                    </datalist>
                  </div>

                  <div className="api-key-form">
                    <input
                      type="password"
                      placeholder="Enter API Key"
                      value={apiKey}
                      onChange={(e) => setApiKey(e.target.value)}
                      onKeyDown={(e) =>
                        e.key === "Enter" && handleConfigureAssistant()
                      }
                    />
                    <button onClick={handleConfigureAssistant}>Start</button>
                  </div>

                  <div className="help-text">
                    <small>
                      Don't have an API key?{" "}
                      <a
                        href="https://aistudio.google.com/app/apikey"
                        target="_blank"
                        rel="noreferrer"
                      >
                        Get one here
                      </a>
                    </small>
                  </div>
                </div>
              )}
            </aside>

            <div className="chat-panel">
              <div className="chat-messages">
                {apiKeySet ? (
                  <>
                    {messages.map((msg, idx) => (
                      <div key={idx} className={`message ${msg.role}`}>
                        <div className="message-content">
                          <p>{msg.content}</p>
                          {msg.changes_table && msg.changes_table.length > 0 && (
                            <ChangesTable changes={msg.changes_table} />
                          )}
                          {msg.timestamp && (
                            <small>
                              {new Date(msg.timestamp).toLocaleTimeString([], {
                                hour: "2-digit",
                                minute: "2-digit",
                              })}
                            </small>
                          )}
                        </div>
                      </div>
                    ))}
                    {loading && (
                      <div className="message assistant">
                        <div className="message-content">
                          {aiPhase ? (
                            <AILayerStatus phase={aiPhase} message={aiMessage} />
                          ) : (
                            <TypingIndicator />
                          )}
                        </div>
                      </div>
                    )}
                  </>
                ) : (
                  <div className="chat-placeholder">
                    <p>Enter your API key in the sidebar to start chatting.</p>
                  </div>
                )}
              </div>

              <div className="chat-input">
                <input
                  type="text"
                  placeholder={`Channel ${selectedTrack + 1}: Try "Metallica tone"`}
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleSendMessage()}
                  disabled={!readyForChat || loading}
                />
                <button
                  onClick={handleSendMessage}
                  disabled={!readyForChat || loading || !input.trim()}
                >
                  {loading ? "..." : "Send"}
                </button>
              </div>
            </div>
          </>
        ) : (
          <EqMatchView />
        )}
      </main>
    </div>
  );
}

export default App;
