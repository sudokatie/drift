//! Real-time audio playback using cpal

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::Engine;

/// Real-time audio player
pub struct Player {
    stream: Option<Stream>,
    running: Arc<AtomicBool>,
}

impl Player {
    /// Create a new player for the given engine
    pub fn new() -> Self {
        Self {
            stream: None,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start playing audio from the engine
    pub fn start(&mut self, engine: Arc<Mutex<Engine>>) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("No output device available"))?;

        let config = device.default_output_config()?;
        let sample_format = config.sample_format();
        let stream_config: StreamConfig = config.into();

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        let stream = match sample_format {
            SampleFormat::F32 => self.build_stream::<f32>(&device, &stream_config, engine, running)?,
            SampleFormat::I16 => self.build_stream::<i16>(&device, &stream_config, engine, running)?,
            SampleFormat::U16 => self.build_stream::<u16>(&device, &stream_config, engine, running)?,
            _ => return Err(anyhow!("Unsupported sample format")),
        };

        stream.play()?;
        self.stream = Some(stream);

        Ok(())
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.stream = None;
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn build_stream<T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>>(
        &self,
        device: &Device,
        config: &StreamConfig,
        engine: Arc<Mutex<Engine>>,
        running: Arc<AtomicBool>,
    ) -> Result<Stream> {
        let channels = config.channels as usize;

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                if !running.load(Ordering::SeqCst) {
                    // Fill with silence when stopped
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0f32);
                    }
                    return;
                }

                if let Ok(mut eng) = engine.try_lock() {
                    for frame in data.chunks_mut(channels) {
                        let sample = eng.process() as f32;
                        for channel_sample in frame.iter_mut() {
                            *channel_sample = T::from_sample(sample);
                        }
                    }
                } else {
                    // Mutex locked, fill with silence
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0f32);
                    }
                }
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;

        Ok(stream)
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the default output device name
pub fn default_device_name() -> Option<String> {
    let host = cpal::default_host();
    host.default_output_device()
        .and_then(|d| d.name().ok())
}

/// List all available output devices
pub fn list_output_devices() -> Vec<(String, StreamConfig)> {
    let host = cpal::default_host();
    let mut devices = Vec::new();

    if let Ok(output_devices) = host.output_devices() {
        for device in output_devices {
            if let (Ok(name), Ok(config)) = (device.name(), device.default_output_config()) {
                devices.push((name, config.into()));
            }
        }
    }

    devices
}
