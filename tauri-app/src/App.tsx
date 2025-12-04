import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { EqMatchView } from "./eq-match/EqMatchView";
import { ChangesTable } from "./components/ChangesTable";
import { AILayerStatus, AILayerPhase } from "./components/AILayerStatus";
import { Toast, ToastMessage } from "./components/Toast";
import { TypingIndicator } from "./components/TypingIndicator";
import { UndoRedo } from "./components/UndoRedo";
import { RecentTones } from "./components/RecentTones";
import { ThemeToggle, useTheme } from "./components/ThemeToggle";
import { FxSearch } from "./components/FxSearch";
import { Tooltip } from "./components/Tooltip";
import { SkeletonChannels, SkeletonFxList } from "./components/Skeleton";
import { useNotificationSound } from "./hooks/useNotificationSound";
import { ChatResponse, ChangeEntry, SecureConfig } from "./types";
import "./App.css";

const PROVIDERS = [
  { key: "grok", label: "xAI Grok" },
] as const;

type ProviderKey = (typeof PROVIDERS)[number]["key"];

const MODEL_PRESETS: Record<ProviderKey, string[]> = {
  grok: ["grok-2-latest", "grok-2-vision-latest", "grok-beta"],
};

interface Message {
  role: "user" | "assistant";
  content: string;
  changes_table?: ChangeEntry[];
  timestamp?: number;
  engine_report?: string;
  action_log?: string[];
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
  const [customInstructions, setCustomInstructions] = useState("");
  const [reaperConnected, setReaperConnected] = useState(false);
  const [loading, setLoading] = useState(false);
  const [tracksLoading, setTracksLoading] = useState(true);
  const [aiPhase, setAiPhase] = useState<AILayerPhase | null>(null);
  const [aiMessage, setAiMessage] = useState<string>("");
  const [toasts, setToasts] = useState<ToastMessage[]>([]);
  const [tracks, setTracks] = useState<TrackInfo[]>([]);
  const [selectedTrack, setSelectedTrack] = useState(0);
  const [provider, setProvider] = useState<ProviderKey>("grok");
  const [model, setModel] = useState(MODEL_PRESETS.grok[0]);
  const [activeView, setActiveView] = useState<"assistant" | "eq">("assistant");
  const [autoConfigAttempted, setAutoConfigAttempted] = useState(false);
  const [fxSearchQuery, setFxSearchQuery] = useState("");

  const inputRef = useRef<HTMLInputElement>(null);
  const { theme, toggleTheme } = useTheme();
  const { playSuccess, playError, playNotification } = useNotificationSound();

  const currentTrack =
    tracks.find((track) => track.index === selectedTrack) ?? tracks[0];
  const currentTrackFx = currentTrack?.fx_list ?? [];
  const filteredFx = fxSearchQuery
    ? currentTrackFx.filter((fx) =>
        fx.name.toLowerCase().includes(fxSearchQuery.toLowerCase())
      )
    : currentTrackFx;
  const activeFxCount = currentTrackFx.filter((fx) => fx.enabled).length;
  const readyForChat = apiKeySet && reaperConnected;

  // Toast helper
  const addToast = useCallback((type: ToastMessage["type"], message: string, duration?: number) => {
    const id = `toast-${Date.now()}-${Math.random()}`;
    setToasts((prev) => [...prev, { id, type, message, duration }]);

    // Play sound based on type
    if (type === "success") playSuccess();
    else if (type === "error") playError();
    else playNotification();
  }, [playSuccess, playError, playNotification]);

  const dismissToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  // Load secure config on startup
  useEffect(() => {
    async function loadSecureConfig() {
      try {
        const result = await invoke<string>("load_api_config");
        const config: SecureConfig = JSON.parse(result);

        if (config.api_key) setApiKey(config.api_key);
        if (config.provider) setProvider(config.provider as ProviderKey);
        if (config.model) setModel(config.model);
        if (config.custom_instructions) setCustomInstructions(config.custom_instructions);
      } catch {
        // No saved config, that's fine
      }
    }

    loadSecureConfig();

    // Load chat history from localStorage
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

  // Save API config securely when it changes
  useEffect(() => {
    if (apiKeySet && apiKey) {
      invoke("save_api_config", {
        apiKey,
        provider,
        model,
        customInstructions: customInstructions || null,
      }).catch(console.error);
    }
  }, [apiKeySet, apiKey, provider, model, customInstructions]);

  useEffect(() => {
    checkReaperConnection();
    const interval = setInterval(checkReaperConnection, 5000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (
      reaperConnected &&
      !apiKeySet &&
      apiKey &&
      model &&
      !autoConfigAttempted
    ) {
      handleConfigureAssistant(true);
      setAutoConfigAttempted(true);
    }
  }, [reaperConnected, apiKey, model, apiKeySet, autoConfigAttempted]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+N or Cmd+N to focus chat input
      if ((e.ctrlKey || e.metaKey) && e.key === "n") {
        e.preventDefault();
        inputRef.current?.focus();
      }
      // Ctrl+Shift+T to toggle theme
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === "T") {
        e.preventDefault();
        toggleTheme();
      }
      // Ctrl+S to save preset
      if ((e.ctrlKey || e.metaKey) && e.key === "s" && reaperConnected) {
        e.preventDefault();
        handleSavePreset();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [reaperConnected, toggleTheme]);

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
    setTracksLoading(true);
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
    } finally {
      setTracksLoading(false);
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

  async function handleConfigureAssistant(silent = false) {
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

      // Save securely
      await invoke("save_api_config", {
        apiKey,
        provider,
        model,
        customInstructions: customInstructions || null,
      });

      addToast(
        "success",
        silent
          ? "Restored saved AI settings."
          : "AI Assistant configured successfully!"
      );
    } catch (error) {
      if (!silent) {
        addToast("error", "Failed to configure AI provider: " + error);
      }
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

    // AI Phase simulation
    const simulatePhases = async () => {
      const toneKeywords = ["tone", "sound", "tonu", "ses"];
      const hasToneRequest = toneKeywords.some(keyword =>
        userInput.toLowerCase().includes(keyword)
      );

      if (hasToneRequest) {
        setAiPhase("detecting");
        setAiMessage("Analyzing your tone request...");
        await new Promise(resolve => setTimeout(resolve, 800));

        setAiPhase("researching");
        setAiMessage("Searching internet for tone details...");
        await new Promise(resolve => setTimeout(resolve, 2000));

        setAiPhase("implementing");
        setAiMessage("Matching plugins to research results...");
        await new Promise(resolve => setTimeout(resolve, 1500));
      } else {
        setAiPhase("implementing");
        setAiMessage("Processing your request...");
        await new Promise(resolve => setTimeout(resolve, 1000));
      }

      setAiPhase("optimizing");
      setAiMessage("AI Engine optimizations...");
      await new Promise(resolve => setTimeout(resolve, 1000));

      setAiPhase("applying");
      setAiMessage("Setting parameters in REAPER...");
    };

    try {
      const phasePromise = simulatePhases();

      const responseString = await invoke<string>("process_chat_message", {
        message: payload,
        track: selectedTrack,
        customInstructions,
      });

      await phasePromise;

      const response: ChatResponse = JSON.parse(responseString);

      setAiPhase("done");
      setAiMessage("Tone created successfully!");
      await new Promise(resolve => setTimeout(resolve, 1000));

      const assistantMessage: Message = {
        role: "assistant",
        content: response.summary,
        changes_table: response.changes_table,
        engine_report: response.engine_report,
        action_log: response.action_log,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, assistantMessage]);
      await loadTrackOverview();

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

  async function handleClearApiConfig() {
    try {
      await invoke("delete_api_config");
    } catch {
      // ignore
    }
    setApiKey("");
    setApiKeySet(false);
    setCustomInstructions("");
    addToast("info", "API settings cleared");
  }

  async function handleSavePreset() {
    const presetName = prompt("Preset name:");
    if (!presetName) return;

    try {
      await invoke<string>("save_preset", { name: presetName });
      addToast("success", `Preset saved: ${presetName}`, 6000);
    } catch (error) {
      addToast("error", "Failed to save preset: " + error);
    }
  }

  async function handleExportTone() {
    const lastAssistantMsg = [...messages].reverse().find(m => m.role === "assistant" && m.changes_table?.length);
    if (!lastAssistantMsg?.changes_table) {
      addToast("warning", "No recent tone changes to export");
      return;
    }

    try {
      const text = await invoke<string>("export_tone_as_text", {
        changes: lastAssistantMsg.changes_table,
        summary: lastAssistantMsg.content,
      });

      // Copy to clipboard
      await navigator.clipboard.writeText(text);
      addToast("success", "Tone settings copied to clipboard!");
    } catch (error) {
      addToast("error", "Failed to export: " + error);
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

  function handleSelectRecentTone(query: string) {
    setInput(query);
    inputRef.current?.focus();
  }

  function handleUndoRedoAction(_action: "undo" | "redo", description: string) {
    addToast("info", description, 3000);
    loadTrackOverview();
  }

  return (
    <div className="app-container">
      <Toast toasts={toasts} onDismiss={dismissToast} />
      <header className="app-header">
        <div className="header-content">
          <h1>🎸 ToneForge</h1>
          <div className="header-actions">
            <UndoRedo onAction={handleUndoRedoAction} />
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
            <Tooltip content="Save current preset (Ctrl+S)" position="bottom">
              <button
                className="primary-btn"
                onClick={handleSavePreset}
                disabled={!reaperConnected}
              >
                Save Preset
              </button>
            </Tooltip>
            <Tooltip content="Export last tone as text" position="bottom">
              <button
                className="ghost-btn"
                onClick={handleExportTone}
                disabled={!messages.some(m => m.changes_table?.length)}
              >
                Export
              </button>
            </Tooltip>
            <ThemeToggle theme={theme} onToggle={toggleTheme} />
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
                      <h3>Assistant Profile</h3>
                      <button className="ghost-btn" onClick={handleClearApiConfig}>
                        Forget key
                      </button>
                    </div>
                    <div className="config-badges">
                      <span className="badge">{provider}</span>
                      <span className="badge">{model}</span>
                      <span className="badge success">Encrypted</span>
                    </div>
                    <div className="custom-instructions">
                      <label htmlFor="custom-instructions">Custom instructions</label>
                      <textarea
                        id="custom-instructions"
                        placeholder="Tell the AI how to speak, what to prioritize, or how to route FX..."
                        value={customInstructions}
                        onChange={(e) => setCustomInstructions(e.target.value)}
                        rows={4}
                      />
                      <small>Applied to every AI request and saved securely.</small>
                    </div>
                    <RecentTones onSelectTone={handleSelectRecentTone} />
                  </div>

                  <div className="sidebar-section">
                    <div className="section-header">
                      <h3>Channels</h3>
                      <button className="ghost-btn" onClick={loadTrackOverview}>
                        Refresh
                      </button>
                    </div>
                    {tracksLoading ? (
                      <SkeletonChannels />
                    ) : tracks.length === 0 ? (
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
                    {currentTrackFx.length > 3 && (
                      <FxSearch onSearch={setFxSearchQuery} placeholder="Search plugins..." />
                    )}
                    {tracksLoading ? (
                      <SkeletonFxList />
                    ) : currentTrack && filteredFx.length > 0 ? (
                      <ul className="fx-list">
                        {filteredFx.map((fx) => (
                          <Tooltip
                            key={fx.index}
                            content={`${fx.name}\n${fx.enabled ? "Active" : "Bypassed"}`}
                            position="right"
                          >
                            <li
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
                          </Tooltip>
                        ))}
                      </ul>
                    ) : (
                      <p className="empty-state">
                        {fxSearchQuery ? "No plugins match your search" : "No plugins loaded"}
                      </p>
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
                      placeholder="grok-2-latest"
                      value={model}
                      onChange={(e) => setModel(e.target.value)}
                    />
                    <datalist id="model-options">
                      {MODEL_PRESETS[provider].map((entry) => (
                        <option key={entry} value={entry} />
                      ))}
                    </datalist>
                  </div>

                  <div className="api-config">
                    <label htmlFor="custom-instructions-setup">Custom instructions</label>
                    <textarea
                      id="custom-instructions-setup"
                      placeholder="e.g. Favor analog amp sims, keep vocals untouched, respond in Turkish."
                      value={customInstructions}
                      onChange={(e) => setCustomInstructions(e.target.value)}
                      rows={3}
                    />
                    <small>Saved securely with your API key and sent with every request.</small>
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
                    <button onClick={() => handleConfigureAssistant()}>Start</button>
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
                          {(msg.engine_report || (msg.action_log && msg.action_log.length > 0)) && (
                            <details
                              className="ai-engine-report"
                              open={idx === messages.length - 1}
                            >
                              <summary>AI process details</summary>
                              {msg.engine_report && (
                                <pre className="engine-report-text">{msg.engine_report}</pre>
                              )}
                              {msg.action_log && msg.action_log.length > 0 && (
                                <ul className="action-log">
                                  {msg.action_log.map((entry, logIdx) => (
                                    <li key={`${idx}-log-${logIdx}`}>{entry}</li>
                                  ))}
                                </ul>
                              )}
                            </details>
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
                  ref={inputRef}
                  type="text"
                  placeholder={`Channel ${selectedTrack + 1}: Try "Metallica tone" (Ctrl+N to focus)`}
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
