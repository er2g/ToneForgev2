import { Line } from 'react-chartjs-2';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Tooltip,
  Legend,
  Filler,
} from 'chart.js';
import type { EQProfile, MatchResult } from '../types';

ChartJS.register(CategoryScale, LinearScale, PointElement, LineElement, Tooltip, Legend, Filler);

interface EQVisualizationProps {
  referenceProfile: EQProfile;
  inputProfile: EQProfile;
  matchResult: MatchResult;
}

export function EQVisualization({ referenceProfile, inputProfile, matchResult }: EQVisualizationProps) {
  const frequencies = referenceProfile.bands.map((band) => band.frequency);

  const chartData = {
    labels: frequencies.map((value) => formatFrequency(value)),
    datasets: [
      {
        label: 'Reference (normalized)',
        data: matchResult.reference_normalized,
        borderColor: '#4ade80',
        backgroundColor: 'rgba(74, 222, 128, 0.1)',
        borderWidth: 2,
        fill: false,
        tension: 0.4,
        pointRadius: 3,
      },
      {
        label: 'Your Audio (normalized)',
        data: matchResult.input_normalized,
        borderColor: '#f87171',
        backgroundColor: 'rgba(248, 113, 113, 0.1)',
        borderWidth: 2,
        fill: false,
        tension: 0.4,
        pointRadius: 3,
      },
      {
        label: 'EQ Correction',
        data: matchResult.correction_profile.bands.map((band) => band.gain_db),
        borderColor: '#fbbf24',
        backgroundColor: 'rgba(251, 191, 36, 0.2)',
        borderWidth: 3,
        fill: true,
        tension: 0.4,
        pointRadius: 5,
        pointHoverRadius: 7,
        borderDash: [5, 5],
      },
    ],
  };

  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    interaction: { mode: 'index' as const, intersect: false },
    plugins: {
      legend: {
        display: true,
        position: 'top' as const,
        labels: { usePointStyle: true, padding: 20, font: { size: 13 } },
      },
      tooltip: {
        backgroundColor: 'rgba(0, 0, 0, 0.85)',
        padding: 12,
        titleFont: { size: 14, weight: 'bold' as const },
        bodyFont: { size: 13 },
        callbacks: {
          title: (context: any) => {
            const freq = frequencies[context[0].dataIndex];
            return `${formatFrequency(freq)} Hz`;
          },
          label: (context: any) => {
            const label = context.dataset.label || '';
            const value = context.parsed.y.toFixed(2);
            return `${label}: ${value > 0 ? '+' : ''}${value} dB`;
          },
        },
      },
    },
    scales: {
      x: {
        title: { display: true, text: 'Frequency (Hz)' },
        grid: { color: 'rgba(255, 255, 255, 0.1)' },
      },
      y: {
        title: { display: true, text: 'Gain (dB)' },
        grid: { color: 'rgba(255, 255, 255, 0.1)' },
        ticks: { callback: (value: any) => `${value > 0 ? '+' : ''}${value}` },
      },
    },
  };

  return (
    <div className="eq-visualization">
      <h2>EQ Match Result</h2>

      <div className="main-chart">
        <Line data={chartData} options={chartOptions} />
      </div>

      <div className="eq-details">
        <div className="eq-bands-grid">
          {matchResult.correction_profile.bands.map((band, index) => (
            <EQBandCard key={band.frequency} band={band} index={index} />
          ))}
        </div>
      </div>

      <div className="comparison-stats">
        <ComparisonStat
          label="Spectral Centroid Shift"
          reference={referenceProfile.spectral_centroid}
          input={inputProfile.spectral_centroid}
          unit="Hz"
        />
        <ComparisonStat
          label="Dynamic Range"
          reference={referenceProfile.dynamic_range}
          input={inputProfile.dynamic_range}
          unit="dB"
        />
        <ComparisonStat
          label="Rolloff Point"
          reference={referenceProfile.spectral_rolloff}
          input={inputProfile.spectral_rolloff}
          unit="Hz"
        />
      </div>
    </div>
  );
}

function EQBandCard({ band, index }: { band: EQProfile['bands'][number]; index: number }) {
  const isBoost = band.gain_db > 0;
  const isCut = band.gain_db < 0;

  return (
    <div className={`eq-band-card ${isBoost ? 'boost' : isCut ? 'cut' : 'neutral'}`}>
      <div className="band-header">
        <span className="band-number">Band {index + 1}</span>
        <span className="band-confidence" title="Analysis confidence">
          {(band.confidence * 100).toFixed(0)}%
        </span>
      </div>

      <div className="band-frequency">{formatFrequency(band.frequency)} Hz</div>

      <div className="band-gain">
        <span className={`gain-value ${isBoost ? 'boost' : isCut ? 'cut' : ''}`}>
          {band.gain_db > 0 ? '+' : ''}
          {band.gain_db.toFixed(2)} dB
        </span>
      </div>

      <div className="band-visual">
        <div className="gain-bar-container">
          <div
            className={`gain-bar ${isBoost ? 'boost' : isCut ? 'cut' : ''}`}
            style={{
              width: `${(Math.abs(band.gain_db) / 12) * 100}%`,
              marginLeft: isBoost ? '50%' : `${50 - (Math.abs(band.gain_db) / 12) * 50}%`,
            }}
          />
          <div className="zero-line" />
        </div>
      </div>

      <div className="band-q">Q: {calculateQ(band.frequency, band.bandwidth).toFixed(2)}</div>
    </div>
  );
}

function ComparisonStat({
  label,
  reference,
  input,
  unit,
}: {
  label: string;
  reference: number;
  input: number;
  unit: string;
}) {
  const diff = reference - input;
  const percentDiff = (diff / reference) * 100;

  return (
    <div className="comparison-stat">
      <h4>{label}</h4>
      <div className="stat-values">
        <div className="stat-row">
          <span className="stat-label">Reference:</span>
          <span className="stat-value">{reference.toFixed(1)} {unit}</span>
        </div>
        <div className="stat-row">
          <span className="stat-label">Your Audio:</span>
          <span className="stat-value">{input.toFixed(1)} {unit}</span>
        </div>
        <div className="stat-row difference">
          <span className="stat-label">Difference:</span>
          <span className={`stat-value ${Math.abs(percentDiff) > 10 ? 'significant' : ''}`}>
            {diff > 0 ? '+' : ''}
            {diff.toFixed(1)} {unit} ({percentDiff > 0 ? '+' : ''}
            {percentDiff.toFixed(1)}%)
          </span>
        </div>
      </div>
    </div>
  );
}

function formatFrequency(freq: number): string {
  if (freq >= 1000) {
    return `${(freq / 1000).toFixed(freq >= 10000 ? 0 : 1)}k`;
  }
  return `${freq.toFixed(0)}`;
}

function calculateQ(centerFreq: number, bandwidth: number): number {
  return centerFreq / bandwidth;
}
