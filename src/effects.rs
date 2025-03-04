#[derive(Default)]
pub struct EffectsState {
    pub amplify_gain: f32,
    pub anti_distortion_threshold: f32,
    pub noise_reduction_threshold: f32,
    pub show_amplify_modal: bool,
    pub show_anti_distortion_modal: bool,
    pub show_noise_reduction_modal: bool,
}

// Amplification effect
pub fn amplify(samples: &mut [f32], gain: f32) {
    for sample in samples.iter_mut() {
        *sample *= gain;
    }
}

// Anti-distortion (limiter) effect
pub fn anti_distortion(samples: &mut [f32], threshold: f32) {
    for sample in samples.iter_mut() {
        if *sample > threshold {
            *sample = threshold;
        } else if *sample < -threshold {
            *sample = -threshold;
        }
    }
}

// Noise reduction using a simple threshold-based gate
pub fn noise_reduction(samples: &mut [f32], threshold: f32) {
    for sample in samples.iter_mut() {
        if sample.abs() < threshold {
            *sample = 0.0;
        }
    }
}

// Compute the RMS (Root Mean Square) of an audio buffer
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f32 = samples.iter().map(|sample| sample * sample).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

// Compute the peak amplitude of an audio buffer
pub fn compute_peak(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    samples.iter().map(|sample| sample.abs()).fold(0.0, f32::max)
}

// More sophisticated noise reduction using spectral subtraction technique
// Note: This is a simplified implementation and doesn't use FFT
pub fn spectral_noise_reduction(samples: &mut [f32], threshold: f32, chunk_size: usize) {
    if samples.len() < chunk_size {
        return;
    }

    // Process in chunks
    for chunk_start in (0..samples.len()).step_by(chunk_size) {
        let chunk_end = (chunk_start + chunk_size).min(samples.len());
        let chunk = &mut samples[chunk_start..chunk_end];

        // Compute local RMS
        let rms = compute_rms(chunk);

        // If RMS is below threshold, apply reduction
        if rms < threshold {
            for sample in chunk.iter_mut() {
                *sample *= rms / threshold;
            }
        }
    }
}
