use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleFormat, Stream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::audio::AudioData;

pub struct PlaybackManager {
    stream: Option<Stream>,
    last_update: Instant,
}

impl PlaybackManager {
    pub fn new() -> Self {
        Self {
            stream: None,
            last_update: Instant::now(),
        }
    }

    pub fn toggle_playback(&mut self, audio_data: Arc<Mutex<AudioData>>) -> Result<(), String> {
        // Check if audio is already playing
        let mut audio = audio_data.lock().unwrap();
        
        if audio.playing {
            // Stop playback
            self.stop_playback();
            audio.playing = false;
            return Ok(());
        }
        
        // Start playback
        if audio.samples.is_empty() {
            return Err("No audio loaded".to_string());
        }
        
        // Set as playing before starting the stream
        audio.playing = true;
        let position = audio.playback_position;
        drop(audio);  // Release lock before starting stream
        
        match self.start_playback(audio_data.clone(), position) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Reset playing flag on error
                audio_data.lock().unwrap().playing = false;
                Err(e)
            }
        }
    }
    
    pub fn stop_playback(&mut self) {
        self.stream = None;
    }

    fn start_playback(&mut self, audio_data: Arc<Mutex<AudioData>>, start_position: usize) -> Result<(), String> {
        let host = cpal::default_host();
        
        let device = host.default_output_device()
            .ok_or("No output device available")?;
        
        // Get audio specs
        let audio = audio_data.lock().unwrap();
        let sample_rate = audio.sample_rate;
        let channels = audio.channels;
        drop(audio);
        
        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };
        
        // Reset the update timer
        self.last_update = Instant::now();
        
        let err_fn = |err| eprintln!("an error occurred on the output audio stream: {}", err);
        
        let stream_result = match device.default_output_config().map(|format| format.sample_format()) {
            Ok(SampleFormat::F32) => self.create_stream::<f32>(device, &config, audio_data, start_position, err_fn),
            Ok(SampleFormat::I16) => self.create_stream::<i16>(device, &config, audio_data, start_position, err_fn),
            Ok(SampleFormat::U16) => self.create_stream::<u16>(device, &config, audio_data, start_position, err_fn),
            _ => return Err("Unsupported sample format".to_string()),
        };
        
        match stream_result {
            Ok(stream) => {
                stream.play().expect("TODO: panic message");
                self.stream = Some(stream);
                Ok(())
            },
            Err(e) => Err(format!("Error creating audio stream: {}", e)),
        }
    }
    
    fn create_stream<T>(&self, device: cpal::Device, config: &cpal::StreamConfig, 
                        audio_data: Arc<Mutex<AudioData>>, start_position: usize,
                        err_fn: impl FnMut(cpal::StreamError) + Send + 'static) -> Result<Stream, cpal::BuildStreamError>
    where
        T: Sample + cpal::FromSample<f32> + cpal::SizedSample,
    {
        device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut audio = audio_data.lock().unwrap();
                
                if !audio.playing || audio.samples.is_empty() {
                    // Clear the buffer or fill with silence if not playing
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0);
                    }
                    return;
                }
                
                // Fill the output buffer with audio data
                for frame in data.chunks_mut(audio.channels as usize) {
                    if audio.playback_position >= audio.samples.len() {
                        // Reached the end of the audio file
                        audio.playback_position = 0;
                        audio.playing = false;
                        
                        // Fill remaining buffer with silence
                        for sample in frame.iter_mut() {
                            *sample = T::from_sample(0.0);
                        }
                        break;
                    }
                    
                    // Copy samples for all channels in this frame
                    for (i, sample) in frame.iter_mut().enumerate() {
                        let channel_idx = i % audio.channels as usize;
                        let sample_idx = audio.playback_position + channel_idx;
                        
                        if sample_idx < audio.samples.len() {
                            *sample = T::from_sample(audio.samples[sample_idx]);
                        } else {
                            *sample = T::from_sample(0.0);
                        }
                    }
                    
                    // Move to the next frame
                    audio.playback_position += audio.channels as usize;
                }
            },
            err_fn,
            None
        )
    }
    
    pub fn update_ui(&mut self, audio_data: &Arc<Mutex<AudioData>>) {
        // This function should be called from the main UI update loop
        if self.last_update.elapsed() > Duration::from_millis(100) {
            self.last_update = Instant::now();
            
            let mut audio = audio_data.lock().unwrap();
            if audio.playing && audio.playback_position >= audio.samples.len() {
                // If we've reached the end (might happen between UI updates)
                audio.playing = false;
                audio.playback_position = 0;
            }
        }
    }
}
