import { useState } from 'react';
import type { MatchConfig } from '../types';

interface ControlPanelProps {
  config: MatchConfig;
  onChange: (config: MatchConfig) => void;
}

export function ControlPanel({ config, onChange }: ControlPanelProps) {
  const [showAdvanced, setShowAdvanced] = useState(false);

  const updateConfig = (updates: Partial<MatchConfig>) => {
    onChange({ ...config, ...updates });
  };

  return (
    <div className="control-panel">
      <h3>Match Settings</h3>

      <div className="control-section">
        <div className="control-group">
          <label>
            <span className="label-text">Match Intensity</span>
            <span className="label-value">{(config.intensity * 100).toFixed(0)}%</span>
          </label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={config.intensity}
            onChange={(event) => updateConfig({ intensity: parseFloat(event.target.value) })}
            className="slider"
          />
          <div className="slider-labels">
            <span>Subtle</span>
            <span>Balanced</span>
            <span>Bold</span>
          </div>
          <p className="help-text">
            {config.intensity < 0.4 && 'Light touch for gentle corrections'}
            {config.intensity >= 0.4 && config.intensity < 0.7 && 'Balanced match for most material'}
            {config.intensity >= 0.7 && 'Maximum correction for close matches'}
          </p>
        </div>

        <div className="control-group">
          <label>
            <span className="label-text">Maximum Correction</span>
            <span className="label-value">{config.max_correction.toFixed(1)} dB</span>
          </label>
          <input
            type="range"
            min="3"
            max="12"
            step="0.5"
            value={config.max_correction}
            onChange={(event) => updateConfig({ max_correction: parseFloat(event.target.value) })}
            className="slider"
          />
          <div className="slider-labels">
            <span>3 dB</span>
            <span>6 dB</span>
            <span>12 dB</span>
          </div>
          <p className="help-text">Limits how far each band can boost or cut.</p>
        </div>

        <div className="control-group">
          <label>
            <span className="label-text">Smoothing</span>
            <span className="label-value">{(config.smoothing_factor * 100).toFixed(0)}%</span>
          </label>
          <input
            type="range"
            min="0"
            max="1"
            step="0.1"
            value={config.smoothing_factor}
            onChange={(event) => updateConfig({ smoothing_factor: parseFloat(event.target.value) })}
            className="slider"
          />
          <div className="slider-labels">
            <span>Sharp</span>
            <span>Smooth</span>
          </div>
          <p className="help-text">Higher values create gentler transitions.</p>
        </div>
      </div>

      <button
        className="btn-text"
        onClick={() => setShowAdvanced((value) => !value)}
        type="button"
      >
        {showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options'}
      </button>

      {showAdvanced && (
        <div className="control-section advanced">
          <div className="toggle-group">
            <label className="toggle-label">
              <input
                type="checkbox"
                checked={config.use_psychoacoustic}
                onChange={(event) => updateConfig({ use_psychoacoustic: event.target.checked })}
              />
              <span className="toggle-switch" />
              <span className="toggle-text">
                <strong>Psychoacoustic Weighting</strong>
                <small>Prioritizes mid-range where ears are most sensitive.</small>
              </span>
            </label>
          </div>

          <div className="toggle-group">
            <label className="toggle-label">
              <input
                type="checkbox"
                checked={config.preserve_dynamics}
                onChange={(event) => updateConfig({ preserve_dynamics: event.target.checked })}
              />
              <span className="toggle-switch" />
              <span className="toggle-text">
                <strong>Preserve Dynamics</strong>
                <small>Keeps natural punch and avoids over-tightening.</small>
              </span>
            </label>
          </div>

          <div className="preset-buttons">
            <h4>Quick Presets</h4>
            <div className="preset-grid">
              <button
                className="btn-preset"
                onClick={() =>
                  onChange({
                    intensity: 0.3,
                    max_correction: 3,
                    smoothing_factor: 0.7,
                    use_psychoacoustic: true,
                    preserve_dynamics: true,
                  })
                }
                type="button"
              >
                Subtle
              </button>
              <button
                className="btn-preset"
                onClick={() =>
                  onChange({
                    intensity: 0.7,
                    max_correction: 6,
                    smoothing_factor: 0.5,
                    use_psychoacoustic: true,
                    preserve_dynamics: true,
                  })
                }
                type="button"
              >
                Balanced
              </button>
              <button
                className="btn-preset"
                onClick={() =>
                  onChange({
                    intensity: 0.9,
                    max_correction: 9,
                    smoothing_factor: 0.3,
                    use_psychoacoustic: true,
                    preserve_dynamics: false,
                  })
                }
                type="button"
              >
                Aggressive
              </button>
              <button
                className="btn-preset"
                onClick={() =>
                  onChange({
                    intensity: 0.6,
                    max_correction: 6,
                    smoothing_factor: 0.8,
                    use_psychoacoustic: true,
                    preserve_dynamics: true,
                  })
                }
                type="button"
              >
                Guitar Focus
              </button>
              <button
                className="btn-preset"
                onClick={() =>
                  onChange({
                    intensity: 0.5,
                    max_correction: 4,
                    smoothing_factor: 0.6,
                    use_psychoacoustic: true,
                    preserve_dynamics: true,
                  })
                }
                type="button"
              >
                Vocal Focus
              </button>
              <button
                className="btn-preset"
                onClick={() =>
                  onChange({
                    intensity: 0.8,
                    max_correction: 8,
                    smoothing_factor: 0.4,
                    use_psychoacoustic: false,
                    preserve_dynamics: false,
                  })
                }
                type="button"
              >
                Mastering Push
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
