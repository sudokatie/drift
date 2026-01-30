//! Drone voice implementation
//!
//! A sustained tone generator with multiple detuned oscillators.

use super::{Oscillator, Voice, Waveform};

/// A drone voice with multiple detuned oscillators
pub struct DroneVoice {
    oscillators: Vec<Oscillator>,
    sample_rate: f64,
    pitch: f64,
    amplitude: f64,
    filter_cutoff: f64,
    active: bool,
    
    // Simple low-pass filter state
    filter_state: f64,
}

impl DroneVoice {
    /// Create a new drone voice
    pub fn new(sample_rate: f64) -> Self {
        // Create multiple detuned oscillators
        let oscillators = vec![
            Oscillator::new(Waveform::Sine, 220.0, sample_rate),
            Oscillator::new(Waveform::Sine, 220.0 * 1.002, sample_rate), // +3.5 cents
            Oscillator::new(Waveform::Sine, 220.0 * 0.998, sample_rate), // -3.5 cents
            Oscillator::new(Waveform::Triangle, 110.0, sample_rate),      // Sub oscillator
        ];
        
        Self {
            oscillators,
            sample_rate,
            pitch: 220.0,
            amplitude: 0.7,
            filter_cutoff: 2000.0,
            active: true,
            filter_state: 0.0,
        }
    }
    
    /// Apply a simple one-pole low-pass filter
    fn apply_filter(&mut self, input: f64) -> f64 {
        let rc = 1.0 / (2.0 * std::f64::consts::PI * self.filter_cutoff);
        let dt = 1.0 / self.sample_rate;
        let alpha = dt / (rc + dt);
        
        self.filter_state = self.filter_state + alpha * (input - self.filter_state);
        self.filter_state
    }
    
    fn update_oscillator_frequencies(&mut self) {
        // Main oscillator at pitch
        self.oscillators[0].set_frequency(self.pitch);
        // Detuned oscillators
        self.oscillators[1].set_frequency(self.pitch * 1.002);
        self.oscillators[2].set_frequency(self.pitch * 0.998);
        // Sub oscillator one octave down
        self.oscillators[3].set_frequency(self.pitch * 0.5);
    }
}

impl Voice for DroneVoice {
    fn set_parameter(&mut self, name: &str, value: f64) {
        match name {
            "pitch" | "frequency" => {
                self.pitch = value.clamp(20.0, 20000.0);
                self.update_oscillator_frequencies();
            }
            "amplitude" | "volume" => {
                self.amplitude = value.clamp(0.0, 1.0);
            }
            "filter" | "filter_cutoff" | "cutoff" => {
                self.filter_cutoff = value.clamp(20.0, 20000.0);
            }
            _ => {}
        }
    }
    
    fn get_parameter(&self, name: &str) -> Option<f64> {
        match name {
            "pitch" | "frequency" => Some(self.pitch),
            "amplitude" | "volume" => Some(self.amplitude),
            "filter" | "filter_cutoff" | "cutoff" => Some(self.filter_cutoff),
            _ => None,
        }
    }
    
    fn trigger(&mut self) {
        self.active = true;
    }
    
    fn release(&mut self) {
        self.active = false;
    }
    
    fn is_active(&self) -> bool {
        self.active
    }
    
    fn process(&mut self) -> f64 {
        if !self.active {
            return 0.0;
        }
        
        // Sum all oscillators
        let mut sum = 0.0;
        for osc in &mut self.oscillators {
            sum += osc.generate();
        }
        
        // Normalize and apply amplitude
        let output = (sum / self.oscillators.len() as f64) * self.amplitude;
        
        // Apply filter
        self.apply_filter(output)
    }
    
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.oscillators = vec![
            Oscillator::new(Waveform::Sine, self.pitch, sample_rate),
            Oscillator::new(Waveform::Sine, self.pitch * 1.002, sample_rate),
            Oscillator::new(Waveform::Sine, self.pitch * 0.998, sample_rate),
            Oscillator::new(Waveform::Triangle, self.pitch * 0.5, sample_rate),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drone_voice_creation() {
        let voice = DroneVoice::new(44100.0);
        assert!(voice.is_active());
        assert_eq!(voice.get_parameter("pitch"), Some(220.0));
    }

    #[test]
    fn test_drone_voice_parameter_setting() {
        let mut voice = DroneVoice::new(44100.0);
        
        voice.set_parameter("pitch", 440.0);
        assert_eq!(voice.get_parameter("pitch"), Some(440.0));
        
        voice.set_parameter("amplitude", 0.5);
        assert_eq!(voice.get_parameter("amplitude"), Some(0.5));
        
        voice.set_parameter("filter_cutoff", 1000.0);
        assert_eq!(voice.get_parameter("filter_cutoff"), Some(1000.0));
    }

    #[test]
    fn test_drone_voice_output() {
        let mut voice = DroneVoice::new(44100.0);
        
        // Generate some samples
        let mut samples = Vec::new();
        for _ in 0..100 {
            samples.push(voice.process());
        }
        
        // Should produce non-zero output
        let max = samples.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        assert!(max > 0.0);
    }

    #[test]
    fn test_drone_voice_release() {
        let mut voice = DroneVoice::new(44100.0);
        
        voice.release();
        assert!(!voice.is_active());
        
        // Should produce zero output when released
        let sample = voice.process();
        assert_eq!(sample, 0.0);
    }
}
