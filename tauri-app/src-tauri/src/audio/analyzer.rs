use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

pub struct AnalysisConfig {
    pub fft_size: usize,
    pub window_type: WindowType,
    pub overlap: f32,  // 0.0 - 0.9
    pub frequency_bands: Vec<f32>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            fft_size: 8192,  // High resolution
            window_type: WindowType::BlackmanHarris,
            overlap: 0.75,   // 75% overlap for smooth analysis
            frequency_bands: vec![
                31.5, 63.0, 125.0, 250.0, 500.0, 
                1000.0, 2000.0, 4000.0, 8000.0, 16000.0
            ],
        }
    }
}

#[derive(Clone, Copy)]
pub enum WindowType {
    Hann,
    Hamming,
    BlackmanHarris,  // Best for audio analysis
    FlatTop,         // Best for amplitude accuracy
}

impl WindowType {
    fn generate(&self, size: usize) -> Vec<f32> {
        (0..size)
            .map(|i| {
                let x = i as f32 / size as f32;
                match self {
                    WindowType::Hann => {
                        0.5 * (1.0 - (2.0 * PI * x).cos())
                    }
                    WindowType::Hamming => {
                        0.54 - 0.46 * (2.0 * PI * x).cos()
                    }
                    WindowType::BlackmanHarris => {
                        let a0 = 0.35875;
                        let a1 = 0.48829;
                        let a2 = 0.14128;
                        let a3 = 0.01168;
                        a0 - a1 * (2.0 * PI * x).cos()
                            + a2 * (4.0 * PI * x).cos()
                            - a3 * (6.0 * PI * x).cos()
                    }
                    WindowType::FlatTop => {
                        let a0 = 0.21557895;
                        let a1 = 0.41663158;
                        let a2 = 0.277263158;
                        let a3 = 0.083578947;
                        let a4 = 0.006947368;
                        a0 - a1 * (2.0 * PI * x).cos()
                            + a2 * (4.0 * PI * x).cos()
                            - a3 * (6.0 * PI * x).cos()
                            + a4 * (8.0 * PI * x).cos()
                    }
                }
            })
            .collect()
    }
}

pub struct FrequencySpectrum {
    pub frequencies: Vec<f32>,
    pub magnitudes: Vec<f32>,  // dB
    pub sample_rate: u32,
}

pub fn analyze_spectrum(
    samples: &[f32],
    sample_rate: u32,
    config: &AnalysisConfig,
) -> FrequencySpectrum {
    let window = config.window_type.generate(config.fft_size);
    let hop_size = (config.fft_size as f32 * (1.0 - config.overlap)) as usize;
    
    // Multiple windows for averaging
    let num_windows = (samples.len() - config.fft_size) / hop_size + 1;
    
    let mut accumulated_spectrum = vec![0.0f32; config.fft_size / 2 + 1];
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(config.fft_size);
    
    for window_idx in 0..num_windows {
        let start = window_idx * hop_size;
        let end = (start + config.fft_size).min(samples.len());
        
        if end - start < config.fft_size {
            break;
        }
        
        // Apply window and prepare FFT buffer
        let mut buffer: Vec<Complex<f32>> = samples[start..end]
            .iter()
            .zip(&window)
            .map(|(&s, &w)| Complex::new(s * w, 0.0))
            .collect();
        
        fft.process(&mut buffer);
        
        // Accumulate magnitude spectrum
        for (i, c) in buffer.iter().take(config.fft_size / 2 + 1).enumerate() {
            accumulated_spectrum[i] += c.norm();
        }
    }
    
    // Average and convert to dB
    let frequencies: Vec<f32> = (0..=config.fft_size / 2)
        .map(|i| i as f32 * sample_rate as f32 / config.fft_size as f32)
        .collect();
    
    let magnitudes: Vec<f32> = accumulated_spectrum
        .iter()
        .map(|&mag| {
            let avg_mag = mag / num_windows as f32;
            20.0 * (avg_mag + 1e-10).log10()  // Convert to dB
        })
        .collect();
    
    FrequencySpectrum {
        frequencies,
        magnitudes,
        sample_rate,
    }
}
