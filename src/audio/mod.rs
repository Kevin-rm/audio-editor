pub(crate) mod playback;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

// Custom error type for WAV operations
#[derive(Debug)]
pub enum WavError {
    IoError(io::Error),
    FormatError(&'static str),
}

impl From<io::Error> for WavError {
    fn from(error: io::Error) -> Self {
        WavError::IoError(error)
    }
}

// Implement Display trait for WavError
impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WavError::IoError(err) => write!(f, "I/O error: {}", err),
            WavError::FormatError(msg) => write!(f, "Format error: {}", msg),
        }
    }
}

// Also implement std::error::Error for completeness
impl std::error::Error for WavError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WavError::IoError(err) => Some(err),
            WavError::FormatError(_) => None,
        }
    }
}

// WAV file format constants
const RIFF_CHUNK_ID: [u8; 4] = *b"RIFF";
const WAVE_FORMAT: [u8; 4] = *b"WAVE";
const FMT_CHUNK_ID: [u8; 4] = *b"fmt ";
const DATA_CHUNK_ID: [u8; 4] = *b"data";

// WAV format types
const WAVE_FORMAT_PCM: u16 = 1;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 3;

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
    pub fn load_file(&mut self, path: PathBuf) -> Result<(), WavError> {
        let mut file = File::open(&path)?;

        // Read RIFF header
        let mut riff_header = [0; 4];
        file.read_exact(&mut riff_header)?;
        if riff_header != RIFF_CHUNK_ID {
            return Err(WavError::FormatError("Not a valid RIFF file"));
        }

        // Read file size (minus 8 bytes for RIFF header and size)
        let _file_size = file.read_u32::<LittleEndian>()?;

        // Read WAVE format
        let mut wave_format = [0; 4];
        file.read_exact(&mut wave_format)?;
        if wave_format != WAVE_FORMAT {
            return Err(WavError::FormatError("Not a valid WAVE file"));
        }

        // Find and read fmt chunk
        let mut found_fmt = false;
        let mut found_data = false;
        let mut format_tag = 0;

        while !found_data {
            // Read chunk ID
            let mut chunk_id = [0; 4];
            if file.read_exact(&mut chunk_id).is_err() {
                break; // End of file reached
            }

            // Read chunk size
            let chunk_size = file.read_u32::<LittleEndian>()?;

            match &chunk_id {
                // Format chunk
                id if id == &FMT_CHUNK_ID => {
                    found_fmt = true;

                    // Read format data
                    format_tag = file.read_u16::<LittleEndian>()?;
                    self.channels = file.read_u16::<LittleEndian>()?;
                    self.sample_rate = file.read_u32::<LittleEndian>()?;
                    let _bytes_per_sec = file.read_u32::<LittleEndian>()?;
                    let _block_align = file.read_u16::<LittleEndian>()?;
                    self.bits_per_sample = file.read_u16::<LittleEndian>()?;

                    // Skip any extra format bytes
                    if chunk_size > 16 {
                        let extra_bytes = chunk_size - 16;
                        file.seek(SeekFrom::Current(extra_bytes as i64))?;
                    }
                }

                // Data chunk
                id if id == &DATA_CHUNK_ID => {
                    if (!found_fmt) {
                        return Err(WavError::FormatError(
                            "Found data chunk before format chunk",
                        ));
                    }

                    found_data = true;

                    // Calculate number of samples
                    let bytes_per_sample = self.bits_per_sample / 8;
                    let num_samples = chunk_size / (bytes_per_sample as u32);

                    // Read samples based on format
                    self.samples = match (self.bits_per_sample, format_tag) {
                        (16, WAVE_FORMAT_PCM) => {
                            let mut samples = Vec::with_capacity(num_samples as usize);
                            for _ in 0..num_samples {
                                let sample = file.read_i16::<LittleEndian>()?;
                                samples.push(sample as f32 / 32768.0);
                            }
                            samples
                        }
                        (24, WAVE_FORMAT_PCM) => {
                            let mut samples = Vec::with_capacity(num_samples as usize);
                            for _ in 0..num_samples {
                                // Read 3 bytes and convert to i32
                                let mut bytes = [0; 3];
                                file.read_exact(&mut bytes)?;

                                // Convert to signed 24-bit integer
                                let mut value = ((bytes[2] as i32) << 16)
                                    | ((bytes[1] as i32) << 8)
                                    | (bytes[0] as i32);

                                // Sign extend if negative (MSB is 1)
                                if (value & 0x800000) != 0 {
                                    // Set all the upper bits to 1 (sign extension)
                                    value |= -0x1000000; // This is the correct way to sign-extend
                                }

                                samples.push(value as f32 / 8388608.0);
                            }
                            samples
                        }
                        (32, WAVE_FORMAT_PCM) => {
                            let mut samples = Vec::with_capacity(num_samples as usize);
                            for _ in 0..num_samples {
                                let sample = file.read_i32::<LittleEndian>()?;
                                samples.push(sample as f32 / 2147483648.0);
                            }
                            samples
                        }
                        (32, WAVE_FORMAT_IEEE_FLOAT) => {
                            let mut samples = Vec::with_capacity(num_samples as usize);
                            for _ in 0..num_samples {
                                let sample = file.read_f32::<LittleEndian>()?;
                                samples.push(sample);
                            }
                            samples
                        }
                        _ => return Err(WavError::FormatError("Unsupported audio format")),
                    };
                }

                // Other chunks - skip them
                _ => {
                    file.seek(SeekFrom::Current(chunk_size as i64))?;
                }
            }
        }

        if (!found_data) {
            return Err(WavError::FormatError("No data chunk found"));
        }

        self.file_path = Some(path);
        self.playing = false;
        self.playback_position = 0;
        self.modified = false;

        // Calculate duration
        self.duration_seconds =
            self.samples.len() as f32 / (self.sample_rate as f32 * self.channels as f32);

        // Generate visualization data
        self.generate_visualization_data();

        Ok(())
    }

    pub fn save_file(&self, path: Option<PathBuf>) -> Result<PathBuf, WavError> {
        let save_path = path.unwrap_or_else(|| self.file_path.clone().unwrap());

        let mut file = File::create(&save_path)?;

        // Determine format tag based on bits per sample
        let format_tag = if self.bits_per_sample == 32 {
            WAVE_FORMAT_IEEE_FLOAT
        } else {
            WAVE_FORMAT_PCM
        };

        // Calculate sizes
        let bytes_per_sample = self.bits_per_sample / 8;
        let data_size = (self.samples.len() * bytes_per_sample as usize) as u32;
        let file_size = 4 + (8 + 16) + (8 + data_size); // WAVE + (fmt chunk) + (data chunk)

        // Write RIFF header
        file.write_all(&RIFF_CHUNK_ID)?;
        file.write_u32::<LittleEndian>(file_size)?;
        file.write_all(&WAVE_FORMAT)?;

        // Write format chunk
        file.write_all(&FMT_CHUNK_ID)?;
        file.write_u32::<LittleEndian>(16)?; // fmt chunk size
        file.write_u16::<LittleEndian>(format_tag)?;
        file.write_u16::<LittleEndian>(self.channels)?;
        file.write_u32::<LittleEndian>(self.sample_rate)?;

        let block_align = self.channels * bytes_per_sample;
        let bytes_per_sec = self.sample_rate * block_align as u32;

        file.write_u32::<LittleEndian>(bytes_per_sec)?;
        file.write_u16::<LittleEndian>(block_align)?;
        file.write_u16::<LittleEndian>(self.bits_per_sample)?;

        // Write data chunk header
        file.write_all(&DATA_CHUNK_ID)?;
        file.write_u32::<LittleEndian>(data_size)?;

        // Write samples based on format
        match (self.bits_per_sample, format_tag) {
            (16, WAVE_FORMAT_PCM) => {
                for sample in &self.samples {
                    let value = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                    file.write_i16::<LittleEndian>(value)?;
                }
            }
            (24, WAVE_FORMAT_PCM) => {
                for sample in &self.samples {
                    let value = (sample.clamp(-1.0, 1.0) * 8388607.0) as i32;
                    // Write 24-bit value as 3 bytes
                    file.write_u8((value & 0xFF) as u8)?;
                    file.write_u8(((value >> 8) & 0xFF) as u8)?;
                    file.write_u8(((value >> 16) & 0xFF) as u8)?;
                }
            }
            (32, WAVE_FORMAT_PCM) => {
                for sample in &self.samples {
                    let value = (sample.clamp(-1.0, 1.0) * 2147483647.0) as i32;
                    file.write_i32::<LittleEndian>(value)?;
                }
            }
            (32, WAVE_FORMAT_IEEE_FLOAT) => {
                for sample in &self.samples {
                    file.write_f32::<LittleEndian>(*sample)?;
                }
            }
            _ => return Err(WavError::FormatError("Unsupported format for saving")),
        }

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
