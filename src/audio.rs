use hound::{Error, WavReader, WavSpec, WavWriter};
use std::path::PathBuf;

#[derive(Default)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub file_path: Option<PathBuf>,
    pub playing: bool,
    pub playback_position: usize,
    pub modified: bool,
    pub visualization_data: Vec<f32>,
    pub duration_seconds: f32,
}

impl AudioData {
    pub fn load_file(&mut self, path: PathBuf) -> Result<(), Error> {
        let mut reader = WavReader::open(&path)?;
        let spec = reader.spec();

        self.sample_rate = spec.sample_rate;
        self.channels = spec.channels;
        self.bits_per_sample = spec.bits_per_sample;
        self.file_path = Some(path);
        self.playing = false;
        self.playback_position = 0;
        self.modified = false;

        // Convert to f32 samples
        self.samples = match (spec.bits_per_sample, spec.sample_format) {
            (16, hound::SampleFormat::Int) => {
                reader.samples::<i16>()
                    .map(|s| s.unwrap_or(0) as f32 / 32768.0)
                    .collect()
            },
            (24, hound::SampleFormat::Int) => {
                reader.samples::<i32>()
                    .map(|s| s.unwrap_or(0) as f32 / 8388608.0)
                    .collect()
            },
            (32, hound::SampleFormat::Int) => {
                reader.samples::<i32>()
                    .map(|s| s.unwrap_or(0) as f32 / 2147483648.0)
                    .collect()
            },
            (32, hound::SampleFormat::Float) => {
                reader.samples::<f32>()
                    .map(|s| s.unwrap_or(0.0))
                    .collect()
            },
            _ => return Err(Error::FormatError("Unsupported audio format")),
        };

        // Calculate duration
        self.duration_seconds = self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32);

        // Generate visualization data (downsampled representation)
        self.generate_visualization_data();

        Ok(())
    }

    pub fn save_file(&self, path: Option<PathBuf>) -> Result<PathBuf, Error> {
        let save_path = path.unwrap_or_else(|| self.file_path.clone().unwrap());

        let spec = WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: self.bits_per_sample,
            sample_format: if self.bits_per_sample == 32 {
                hound::SampleFormat::Float
            } else {
                hound::SampleFormat::Int
            },
        };

        let mut writer = WavWriter::create(&save_path, spec)?;

        // Write samples based on the bit depth
        match (spec.bits_per_sample, spec.sample_format) {
            (16, hound::SampleFormat::Int) => {
                for sample in &self.samples {
                    let sample = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                    writer.write_sample(sample)?;
                }
            },
            (24, hound::SampleFormat::Int) => {
                for sample in &self.samples {
                    let sample = (sample.clamp(-1.0, 1.0) * 8388607.0) as i32;
                    writer.write_sample(sample)?;
                }
            },
            (32, hound::SampleFormat::Int) => {
                for sample in &self.samples {
                    let sample = (sample.clamp(-1.0, 1.0) * 2147483647.0) as i32;
                    writer.write_sample(sample)?;
                }
            },
            (32, hound::SampleFormat::Float) => {
                for sample in &self.samples {
                    writer.write_sample(*sample)?;
                }
            },
            _ => return Err(Error::FormatError("Unsupported audio format for saving")),
        }

        writer.finalize()?;
        Ok(save_path)
    }

    pub fn generate_visualization_data(&mut self) {
        // Create a downsampled representation for visualization
        const MAX_POINTS: usize = 1000;

        let samples_per_channel = self.samples.len() / self.channels as usize;
        let mut visualization = Vec::with_capacity(MAX_POINTS);

        if samples_per_channel <= MAX_POINTS {
            // If we have fewer samples than visualization points, use all samples
            for i in 0..samples_per_channel {
                let mut sum = 0.0;
                for ch in 0..self.channels {
                    sum += self.samples[i * self.channels as usize + ch as usize].abs();
                }
                visualization.push(sum / self.channels as f32);
            }
        } else {
            // Otherwise, downsample
            let step = samples_per_channel as f32 / MAX_POINTS as f32;

            for i in 0..MAX_POINTS {
                let sample_idx = (i as f32 * step) as usize;
                let mut sum = 0.0;
                for ch in 0..self.channels {
                    sum += self.samples[sample_idx * self.channels as usize + ch as usize].abs();
                }
                visualization.push(sum / self.channels as f32);
            }
        }

        self.visualization_data = visualization;
    }
}
