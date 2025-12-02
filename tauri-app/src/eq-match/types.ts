export interface EQBand {
  frequency: number;
  gain_db: number;
  bandwidth: number;
  confidence: number;
}

export interface EQProfile {
  bands: EQBand[];
  overall_loudness: number;
  dynamic_range: number;
  spectral_centroid: number;
  spectral_rolloff: number;
}

export interface MatchResult {
  correction_profile: EQProfile;
  reference_normalized: number[];
  input_normalized: number[];
  quality_score: number;
  warnings: string[];
}

export interface MatchConfig {
  intensity: number;
  max_correction: number;
  smoothing_factor: number;
  use_psychoacoustic: boolean;
  preserve_dynamics: boolean;
}

export type ProcessStep = 'upload' | 'analyze' | 'match' | 'export';
