import { useState, type ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { FileUploader } from './components/FileUploader';
import { FrequencyAnalyzer } from './components/FrequencyAnalyzer';
import { EQVisualization } from './components/EQVisualization';
import { ControlPanel } from './components/ControlPanel';
import { ExportPanel } from './components/ExportPanel';
import type { EQProfile, MatchResult, MatchConfig, ProcessStep } from './types';
import './eq-match.css';

const defaultConfig: MatchConfig = {
  intensity: 0.7,
  max_correction: 6,
  smoothing_factor: 0.5,
  use_psychoacoustic: true,
  preserve_dynamics: true,
};

export function EqMatchView() {
  const [step, setStep] = useState<ProcessStep>('upload');
  const [referenceProfile, setReferenceProfile] = useState<EQProfile | null>(null);
  const [inputProfile, setInputProfile] = useState<EQProfile | null>(null);
  const [matchResult, setMatchResult] = useState<MatchResult | null>(null);
  const [matchConfig, setMatchConfig] = useState<MatchConfig>(defaultConfig);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const selectAudio = async () => {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Audio Files',
          extensions: ['wav', 'mp3', 'flac', 'ogg', 'm4a', 'aac'],
        },
      ],
    });

    if (selected && typeof selected === 'string') {
      return selected;
    }
    return null;
  };

  const handleLoadReference = async () => {
    try {
      setLoading(true);
      setError(null);
      const path = await selectAudio();
      if (!path) return;
      const profile = await invoke<EQProfile>('load_reference_audio', { path });
      setReferenceProfile(profile);
    } catch (err) {
      setError(`Reference load failed: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleLoadInput = async () => {
    try {
      setLoading(true);
      setError(null);
      const path = await selectAudio();
      if (!path) return;
      const profile = await invoke<EQProfile>('load_input_audio', { path });
      setInputProfile(profile);
      setStep('analyze');
    } catch (err) {
      setError(`Input load failed: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCalculateMatch = async () => {
    if (!referenceProfile || !inputProfile) {
      return;
    }

    try {
      setLoading(true);
      setError(null);
      const result = await invoke<MatchResult>('calculate_eq_match', {
        reference: referenceProfile,
        input: inputProfile,
        config: matchConfig,
      });
      setMatchResult(result);
      setStep('match');
    } catch (err) {
      setError(`Match calculation failed: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleExport = async (format: string) => {
    if (!matchResult) {
      return;
    }
    const exported = await invoke<string>('export_eq_settings', {
      result: matchResult,
      format,
    });

    const blob = new Blob([exported], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `eq-match.${format === 'reaper' ? 'RfxChain' : format}`;
    link.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="eqm-root">
      <header className="eqm-header">
        <div>
          <p className="eqm-kicker">ToneForge Utility</p>
          <h1>Precision EQ Matcher</h1>
        </div>
        <div className="eqm-step-indicator">
          <StepBadge active={step === 'upload'} completed={!!referenceProfile}>
            1. Upload
          </StepBadge>
          <StepBadge active={step === 'analyze'} completed={!!inputProfile}>
            2. Analyze
          </StepBadge>
          <StepBadge active={step === 'match'} completed={!!matchResult}>
            3. Match
          </StepBadge>
          <StepBadge active={step === 'export'} completed={false}>
            4. Export
          </StepBadge>
        </div>
      </header>

      <main className="eqm-main">
        {error && (
          <div className="eqm-error">
            <span>{error}</span>
            <button onClick={() => setError(null)} type="button">
              Dismiss
            </button>
          </div>
        )}

        {step === 'upload' && (
          <div className="eqm-upload-section">
            <FileUploader
              title="Reference Audio"
              subtitle="Pick the track you want to sound like"
              onLoad={handleLoadReference}
              loaded={!!referenceProfile}
              loading={loading}
            />

            {referenceProfile && (
              <>
                <div className="eqm-arrow-down" aria-hidden="true">
                  &darr;
                </div>
                <FileUploader
                  title="Your Audio"
                  subtitle="Pick the track you want to correct"
                  onLoad={handleLoadInput}
                  loaded={!!inputProfile}
                  loading={loading}
                />
              </>
            )}
          </div>
        )}

        {step === 'analyze' && referenceProfile && inputProfile && (
          <div className="eqm-analyze-section">
            <div className="eqm-profiles-comparison">
              <FrequencyAnalyzer title="Reference" profile={referenceProfile} color="#4ade80" />
              <FrequencyAnalyzer title="Your Audio" profile={inputProfile} color="#f87171" />
            </div>

            <ControlPanel config={matchConfig} onChange={setMatchConfig} />

            <button
              className="btn-primary btn-large"
              onClick={handleCalculateMatch}
              disabled={loading}
              type="button"
            >
              {loading ? 'Calculating...' : 'Calculate EQ Match'}
            </button>
          </div>
        )}

        {step === 'match' && referenceProfile && inputProfile && matchResult && (
          <div className="eqm-match-section">
            <EQVisualization
              referenceProfile={referenceProfile}
              inputProfile={inputProfile}
              matchResult={matchResult}
            />

            <div className="eqm-match-quality">
              <h3>Match Quality</h3>
              <div className="quality-bar">
                <div
                  className="quality-fill"
                  style={{ width: `${matchResult.quality_score * 100}%`, backgroundColor: getQualityColor(matchResult.quality_score) }}
                />
              </div>
              <span className="quality-score">{(matchResult.quality_score * 100).toFixed(0)}%</span>
            </div>

            {matchResult.warnings.length > 0 && (
              <div className="eqm-warnings">
                <h4>Suggestions</h4>
                <ul>
                  {matchResult.warnings.map((warning, index) => (
                    <li key={index}>{warning}</li>
                  ))}
                </ul>
              </div>
            )}

            <div className="action-buttons">
              <button className="btn-secondary" onClick={() => setStep('analyze')} type="button">
                Adjust Settings
              </button>
              <button className="btn-primary" onClick={() => setStep('export')} type="button">
                Export Settings
              </button>
            </div>
          </div>
        )}

        {step === 'export' && matchResult && (
          <ExportPanel matchResult={matchResult} onExport={handleExport} onBack={() => setStep('match')} />
        )}
      </main>
    </div>
  );
}

function StepBadge({ active, completed, children }: { active: boolean; completed: boolean; children: ReactNode }) {
  return (
    <div className={`eqm-step-badge ${active ? 'active' : ''} ${completed ? 'completed' : ''}`}>
      {children}
    </div>
  );
}

function getQualityColor(score: number) {
  if (score >= 0.8) return '#4ade80';
  if (score >= 0.6) return '#fbbf24';
  return '#f87171';
}
