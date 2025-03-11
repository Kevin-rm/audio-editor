use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

pub struct EffectsState {
    pub amplify_gain: f32,
    pub anti_distortion_threshold: f32,
    pub noise_reduction_threshold: f32,
    pub show_amplify_modal: bool,
    pub show_anti_distortion_modal: bool,
    pub show_noise_reduction_modal: bool,
    pub noise_profile: Vec<f32>, // Stocke le profil de bruit de référence
    pub noise_profile_loaded: bool, // Indique si un profil de bruit est chargé
    pub noise_profile_name: String, // Nom du fichier de profil de bruit
}

impl Default for EffectsState {
    fn default() -> Self {
        Self {
            amplify_gain: 1.0,
            anti_distortion_threshold: 0.8,
            noise_reduction_threshold: 0.01,
            show_amplify_modal: false,
            show_anti_distortion_modal: false,
            show_noise_reduction_modal: false,
            noise_profile: Vec::new(),
            noise_profile_loaded: false,
            noise_profile_name: String::new(),
        }
    }
}

// Apply amplification effect
pub fn amplify(samples: &mut [f32], gain: f32) {
    for sample in samples.iter_mut() {
        *sample *= gain;
    }
}

// Appliquer un anti-distorsion avec un soft knee
pub fn anti_distortion(samples: &mut [f32], threshold: f32) {
    let knee_width = 0.1; // Zone de transition douce

    for sample in samples.iter_mut() {
        let abs_sample = sample.abs();
        if abs_sample > threshold {
            if abs_sample < threshold + knee_width {
                // Appliquer une compression douce (soft knee)
                let excess = abs_sample - threshold;
                let soft_factor = excess / knee_width;
                *sample = (threshold + (1.0 - soft_factor) * excess) * sample.signum();
            } else {
                // Limitation dure au-delà de la zone de transition
                *sample = threshold * sample.signum();
            }
        }
    }
}

// Import or set a noise profile from a sample file
pub fn import_noise_profile(file_path: std::path::PathBuf) -> Result<Vec<f32>, String> {
    // Load the noise sample file
    let reader = match hound::WavReader::open(&file_path) {
        Ok(r) => r,
        Err(e) => return Err(format!("Erreur lors de l'ouverture du fichier: {}", e)),
    };
    
    let spec = reader.spec();
    
    // Convert samples to f32 regardless of the original format
    let noise_profile: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            match spec.bits_per_sample {
                16 => reader.into_samples::<i16>()
                     .map(|s| s.unwrap_or(0) as f32 / i16::MAX as f32)
                     .collect(),
                24 => reader.into_samples::<i32>()
                     .map(|s| s.unwrap_or(0) as f32 / 8388608.0) // 2^23
                     .collect(),
                32 => reader.into_samples::<i32>()
                     .map(|s| s.unwrap_or(0) as f32 / i32::MAX as f32)
                     .collect(),
                _ => return Err(format!("Format d'échantillon non supporté: {} bits", spec.bits_per_sample)),
            }
        },
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>()
                  .map(|s| s.unwrap_or(0.0))
                  .collect()
        },
    };
    
    if noise_profile.is_empty() {
        return Err("Le fichier de bruit est vide".to_string());
    }
    
    Ok(noise_profile)
}

// Adjust noise profile length to match audio data
pub fn adjust_noise_profile(noise_profile: &[f32], target_length: usize) -> Vec<f32> {
    if noise_profile.is_empty() {
        return vec![0.0; target_length];
    }
    
    let mut adjusted_profile = Vec::with_capacity(target_length);
    
    if noise_profile.len() >= target_length {
        // If noise profile is longer or equal, just take what we need
        adjusted_profile.extend_from_slice(&noise_profile[0..target_length]);
    } else {
        // If noise profile is shorter, loop it to fill the target length
        let mut remaining = target_length;
        while remaining > 0 {
            let chunk_size = std::cmp::min(remaining, noise_profile.len());
            adjusted_profile.extend_from_slice(&noise_profile[0..chunk_size]);
            remaining -= chunk_size;
        }
    }
    
    adjusted_profile
}

pub fn noise_cancellation(samples: &mut [f32], noise_profile: &[f32], threshold: f32) {
    if noise_profile.is_empty() || samples.is_empty() {
        return;
    }
    
    // Ensure the noise profile is the right size
    let adjusted_profile = adjust_noise_profile(noise_profile, samples.len());
    
    for (sample, noise) in samples.iter_mut().zip(adjusted_profile.iter()) {
        let diff = *sample - noise;

        if diff.abs() < threshold {
            *sample = 0.0; // Suppression complète si sous le seuil
        } else {
            // Réduction progressive du bruit avec un soft threshold
            *sample = diff.signum() * (diff.abs() - threshold * 0.5);
        }
    }
}

// Simple noise reduction for backward compatibility
pub fn noise_reduction(samples: &mut [f32], threshold: f32) {
    for sample in samples.iter_mut() {
        if sample.abs() < threshold {
            *sample = 0.0;
        }
    }
}

// Calculate peak amplitude
pub fn compute_peak(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s.abs()).fold(0.0, f32::max)
}

// Calculate RMS (Root Mean Square)
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squared: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squared / samples.len() as f32).sqrt()
}
