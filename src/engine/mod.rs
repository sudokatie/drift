//! Audio engine for Drift
//!
//! Manages audio output and voice mixing.

use crate::config::DriftConfig;
use crate::synth::{DroneVoice, Voice};
use anyhow::Result;

/// The main audio engine
pub struct Engine {
    config: DriftConfig,
    voices: Vec<Box<dyn Voice>>,
    sample_rate: f64,
    running: bool,
}

impl Engine {
    /// Create a new engine with the given configuration
    pub fn new(config: DriftConfig) -> Self {
        let sample_rate = config.audio.sample_rate as f64;
        
        Self {
            config,
            voices: Vec::new(),
            sample_rate,
            running: false,
        }
    }
    
    /// Get the sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
    
    /// Add a voice to the engine
    pub fn add_voice(&mut self, voice: Box<dyn Voice>) {
        self.voices.push(voice);
    }
    
    /// Add a drone voice
    pub fn add_drone(&mut self) -> usize {
        let voice = DroneVoice::new(self.sample_rate);
        self.voices.push(Box::new(voice));
        self.voices.len() - 1
    }
    
    /// Set a parameter on a voice
    pub fn set_voice_parameter(&mut self, voice_index: usize, name: &str, value: f64) {
        if let Some(voice) = self.voices.get_mut(voice_index) {
            voice.set_parameter(name, value);
        }
    }
    
    /// Generate the next sample (mix of all voices)
    pub fn process(&mut self) -> f64 {
        let mut output = 0.0;
        
        for voice in &mut self.voices {
            if voice.is_active() {
                output += voice.process();
            }
        }
        
        // Apply master volume
        output * self.config.master.volume as f64
    }
    
    /// Fill a buffer with samples
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process() as f32;
        }
    }
    
    /// Check if the engine is running
    pub fn is_running(&self) -> bool {
        self.running
    }
    
    /// Start the engine
    pub fn start(&mut self) -> Result<()> {
        self.running = true;
        Ok(())
    }
    
    /// Stop the engine
    pub fn stop(&mut self) {
        self.running = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AudioConfig, MasterConfig};
    use std::collections::HashMap;

    fn test_config() -> DriftConfig {
        DriftConfig {
            audio: AudioConfig {
                sample_rate: 44100,
                buffer_size: 512,
                device: None,
                output_file: None,
            },
            master: MasterConfig {
                bpm: 60.0,
                key: "C".to_string(),
                scale: "minor_pentatonic".to_string(),
                volume: 0.7,
            },
            sources: vec![],
            layers: vec![],
        }
    }

    #[test]
    fn test_engine_creation() {
        let config = test_config();
        let engine = Engine::new(config);
        
        assert_eq!(engine.sample_rate(), 44100.0);
        assert!(!engine.is_running());
    }

    #[test]
    fn test_engine_add_drone() {
        let config = test_config();
        let mut engine = Engine::new(config);
        
        let idx = engine.add_drone();
        assert_eq!(idx, 0);
        
        // Generate multiple samples and check for non-zero output
        // (filter may attenuate initial samples)
        let mut max_sample = 0.0f64;
        for _ in 0..1000 {
            let sample = engine.process();
            max_sample = max_sample.max(sample.abs());
        }
        assert!(max_sample > 0.0, "Expected non-zero audio output");
    }

    #[test]
    fn test_engine_fill_buffer() {
        let config = test_config();
        let mut engine = Engine::new(config);
        engine.add_drone();
        
        let mut buffer = vec![0.0f32; 512];
        engine.fill_buffer(&mut buffer);
        
        // Buffer should have non-zero samples
        let has_audio = buffer.iter().any(|&s| s.abs() > 0.0);
        assert!(has_audio);
    }

    #[test]
    fn test_engine_parameter_setting() {
        let config = test_config();
        let mut engine = Engine::new(config);
        let idx = engine.add_drone();
        
        engine.set_voice_parameter(idx, "pitch", 440.0);
        
        // Generate some samples - should work without panicking
        for _ in 0..100 {
            engine.process();
        }
    }
}
