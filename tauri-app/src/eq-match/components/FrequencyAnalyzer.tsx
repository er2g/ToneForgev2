import { Line } from 'react-chartjs-2';
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from 'chart.js';
import type { EQProfile } from '../types';

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler,
);

interface FrequencyAnalyzerProps {
  title: string;
  profile: EQProfile;
  color: string;
}

export function FrequencyAnalyzer({ title, profile, color }: FrequencyAnalyzerProps) {
  const gains = profile.bands.map((band) => band.gain_db);
  const mean = gains.reduce((sum, value) => sum + value, 0) / gains.length;
  const normalizedGains = gains.map((value) => value - mean);

  const chartData = {
    labels: profile.bands.map((band) => formatFrequency(band.frequency)),
    datasets: [
      {
        label: 'Frequency Response',
        data: normalizedGains,
        borderColor: color,
        backgroundColor: `${color}33`,
        fill: true,
        tension: 0.4,
        pointRadius: 4,
        pointHoverRadius: 6,
      },
    ],
  };

  const chartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { display: false },
      tooltip: {
        callbacks: {
          label: (context: any) => `${context.parsed.y.toFixed(2)} dB (normalized)`,
        },
      },
    },
    scales: {
      x: {
        title: { display: true, text: 'Frequency', font: { size: 12, weight: 'bold' as const } },
        grid: { color: 'rgba(255, 255, 255, 0.1)' },
      },
      y: {
        title: { display: true, text: 'Gain (dB)', font: { size: 12, weight: 'bold' as const } },
        grid: { color: 'rgba(255, 255, 255, 0.1)' },
        ticks: { callback: (value: any) => `${value} dB` },
      },
    },
  };

  return (
    <div className="frequency-analyzer">
      <h3 style={{ color }}>{title}</h3>

      <div className="chart-container">
        <Line data={chartData} options={chartOptions} />
      </div>

      <div className="audio-stats">
        <StatItem label="Loudness" value={`${profile.overall_loudness.toFixed(1)} dB`} />
        <StatItem label="Dynamic Range" value={`${profile.dynamic_range.toFixed(1)} dB`} />
        <StatItem label="Spectral Centroid" value={formatFrequency(profile.spectral_centroid)} />
        <StatItem label="Rolloff (85%)" value={formatFrequency(profile.spectral_rolloff)} />
      </div>

      <div className="confidence-indicator">
        <span>Analysis Confidence</span>
        <div className="confidence-bars">
          {profile.bands.map((band, index) => (
            <div
              key={index}
              className="confidence-bar"
              style={{ height: `${band.confidence * 100}%`, backgroundColor: color }}
              title={`${band.frequency} Hz: ${(band.confidence * 100).toFixed(0)}%`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function StatItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat-item">
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
    </div>
  );
}

function formatFrequency(freq: number): string {
  if (freq >= 1000) {
    return `${(freq / 1000).toFixed(freq >= 10000 ? 0 : 1)}k`;
  }
  return `${freq.toFixed(0)}`;
}
