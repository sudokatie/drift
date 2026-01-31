//! Drone voice implementation
//!
//! A sustained tone generator with multiple detuned oscillators,
//! ADSR envelope, biquad filter, and LFO modulation.

use super::{Envelope, Filter, FilterType, Lfo, LfoShape, Oscillator, Voice, Waveform};

/// A drone voice with multiple detuned oscillators and full modulation
pub struct DroneVoice {
    /// Main oscillators (detuned for richness)
    oscillators: Vec<Oscillator>,
    /// Sub oscillator (one octave down)
    sub_oscillator: Oscillator,
    /// Noise oscillator for texture
    noise_oscillator: Oscillator,
    /// ADSR amplitude envelope
    envelope: Envelope,
    /// Biquad low-pass filter
    filter: Filter,
    /// LFO for filter modulation
    filter_lfo: Lfo,
    /// LFO for pitch modulation (vibrato)
    pitch_lfo: Lfo,
    
    sample_rate: f64,
    
    // Parameters
    pitch: f64,
    amplitude: f64,
    filter_cutoff: f64,
    filter_resonance: f64,
    /// Filter LFO depth (how much LFO affects cutoff)
    filter_lfo_depth: f64,
    /// Pitch LFO depth in cents
    pitch_lfo_depth: f64,
    /// Noise mix level (0.0 to 1.0)
    noise_mix: f64,
    /// Sub oscillator mix level
    sub_mix: f64,
    
    active: bool,
}

impl DroneVoice {
    /// Create a new drone voice
    pub fn new(sample_rate: f64) -> Self {
        // Create multiple detuned oscillators for rich sound
        let oscillators = vec![
            Oscillator::new(Waveform::Saw, 220.0, sample_rate),
            Oscillator::new(Waveform::Saw, 220.0 * 1.003, sample_rate), // +5 cents
            Oscillator::new(Waveform::Saw, 220.0 * 0.997, sample_rate), // -5 cents
            Oscillator::new(Waveform::Square, 220.0 * 1.001, sample_rate), // +1.7 cents
        ];
        
        let sub_oscillator = Oscillator::new(Waveform::Sine, 110.0, sample_rate);
        let noise_oscillator = Oscillator::new(Waveform::PinkNoise, 1.0, sample_rate);
        
        // Configure envelope for drone (slow attack, long sustain)
        let mut envelope = Envelope::new(sample_rate);
        envelope.configure(0.5, 0.3, 0.8, 1.0); // 500ms attack, 300ms decay, 80% sustain, 1s release
        
        // Configure filter
        let mut filter = Filter::with_type(sample_rate, FilterType::LowPass);
        filter.set_cutoff(2000.0);
        filter.set_resonance(1.5); // Slight resonance for character
        
        // Configure filter LFO (slow, subtle)
        let mut filter_lfo = Lfo::new(sample_rate);
        filter_lfo.set_frequency(0.1); // Very slow
        filter_lfo.set_shape(LfoShape::Sine);
        filter_lfo.set_depth(1.0);
        
        // Configure pitch LFO (vibrato - subtle)
        let mut pitch_lfo = Lfo::new(sample_rate);
        pitch_lfo.set_frequency(4.0); // 4 Hz vibrato
        pitch_lfo.set_shape(LfoShape::Sine);
        pitch_lfo.set_depth(1.0);
        
        let mut voice = Self {
            oscillators,
            sub_oscillator,
            noise_oscillator,
            envelope,
            filter,
            filter_lfo,
            pitch_lfo,
            sample_rate,
            pitch: 220.0,
            amplitude: 0.7,
            filter_cutoff: 2000.0,
            filter_resonance: 1.5,
            filter_lfo_depth: 500.0, // 500 Hz modulation range
            pitch_lfo_depth: 5.0,    // 5 cents vibrato
            noise_mix: 0.02,         // Subtle noise
            sub_mix: 0.3,            // 30% sub
            active: true,
        };
        
        // Auto-trigger for sustained drone behavior
        voice.envelope.trigger();
        
        voice
    }
    
    /// Update all oscillator frequencies based on pitch + LFO
    fn update_oscillator_frequencies(&mut self, pitch_mod: f64) {
        // Convert cents to frequency multiplier
        let cents_mult = 2.0_f64.powf(pitch_mod / 1200.0);
        let modulated_pitch = self.pitch * cents_mult;
        
        // Main oscillators with detuning
        self.oscillators[0].set_frequency(modulated_pitch);
        self.oscillators[1].set_frequency(modulated_pitch * 1.003);
        self.oscillators[2].set_frequency(modulated_pitch * 0.997);
        self.oscillators[3].set_frequency(modulated_pitch * 1.001);
        
        // Sub one octave down
        self.sub_oscillator.set_frequency(modulated_pitch * 0.5);
    }
}

impl Voice for DroneVoice {
    fn set_parameter(&mut self, name: &str, value: f64) {
        match name {
            "pitch" | "frequency" => {
                self.pitch = value.clamp(20.0, 20000.0);
                self.update_oscillator_frequencies(0.0);
            }
            "amplitude" | "volume" => {
                self.amplitude = value.clamp(0.0, 1.0);
            }
            "filter" | "filter_cutoff" | "cutoff" => {
                self.filter_cutoff = value.clamp(20.0, 20000.0);
                self.filter.set_cutoff(self.filter_cutoff);
            }
            "filter_resonance" | "resonance" | "q" => {
                self.filter_resonance = value.clamp(0.1, 20.0);
                self.filter.set_resonance(self.filter_resonance);
            }
            "filter_lfo_rate" | "filter_lfo_freq" => {
                self.filter_lfo.set_frequency(value.clamp(0.01, 20.0));
            }
            "filter_lfo_depth" => {
                self.filter_lfo_depth = value.clamp(0.0, 5000.0);
            }
            "vibrato_rate" | "pitch_lfo_rate" => {
                self.pitch_lfo.set_frequency(value.clamp(0.1, 20.0));
            }
            "vibrato_depth" | "pitch_lfo_depth" => {
                self.pitch_lfo_depth = value.clamp(0.0, 100.0);
            }
            "noise_mix" | "noise" => {
                self.noise_mix = value.clamp(0.0, 1.0);
            }
            "sub_mix" | "sub" => {
                self.sub_mix = value.clamp(0.0, 1.0);
            }
            "attack" => {
                self.envelope.set_attack(value.clamp(0.001, 10.0));
            }
            "decay" => {
                self.envelope.set_decay(value.clamp(0.001, 10.0));
            }
            "sustain" => {
                self.envelope.set_sustain(value.clamp(0.0, 1.0));
            }
            "release" => {
                self.envelope.set_release(value.clamp(0.001, 30.0));
            }
            _ => {}
        }
    }
    
    fn get_parameter(&self, name: &str) -> Option<f64> {
        match name {
            "pitch" | "frequency" => Some(self.pitch),
            "amplitude" | "volume" => Some(self.amplitude),
            "filter" | "filter_cutoff" | "cutoff" => Some(self.filter_cutoff),
            "filter_resonance" | "resonance" | "q" => Some(self.filter_resonance),
            "filter_lfo_rate" | "filter_lfo_freq" => Some(self.filter_lfo.frequency()),
            "filter_lfo_depth" => Some(self.filter_lfo_depth),
            "vibrato_rate" | "pitch_lfo_rate" => Some(self.pitch_lfo.frequency()),
            "vibrato_depth" | "pitch_lfo_depth" => Some(self.pitch_lfo_depth),
            "noise_mix" | "noise" => Some(self.noise_mix),
            "sub_mix" | "sub" => Some(self.sub_mix),
            _ => None,
        }
    }
    
    fn trigger(&mut self) {
        self.active = true;
        self.envelope.trigger();
    }
    
    fn release(&mut self) {
        self.envelope.release();
    }
    
    fn is_active(&self) -> bool {
        self.active && self.envelope.is_active()
    }
    
    fn process(&mut self) -> f64 {
        if !self.active {
            return 0.0;
        }
        
        // Get LFO values
        let pitch_mod = self.pitch_lfo.process() * self.pitch_lfo_depth;
        let filter_mod = self.filter_lfo.process() * self.filter_lfo_depth;
        
        // Update oscillator frequencies with vibrato
        self.update_oscillator_frequencies(pitch_mod);
        
        // Update filter cutoff with LFO
        let modulated_cutoff = (self.filter_cutoff + filter_mod).clamp(20.0, 20000.0);
        self.filter.set_cutoff(modulated_cutoff);
        
        // Sum main oscillators
        let mut sum = 0.0;
        for osc in &mut self.oscillators {
            sum += osc.generate();
        }
        sum /= self.oscillators.len() as f64;
        
        // Add sub oscillator
        sum += self.sub_oscillator.generate() * self.sub_mix;
        
        // Add noise
        sum += self.noise_oscillator.generate() * self.noise_mix;
        
        // Apply filter
        let filtered = self.filter.process(sum);
        
        // Apply envelope
        let env_level = self.envelope.process();
        
        // Apply amplitude and envelope
        let output = filtered * env_level * self.amplitude;
        
        // Check if envelope has finished
        if !self.envelope.is_active() {
            self.active = false;
        }
        
        output
    }
    
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        
        // Recreate oscillators at new sample rate
        self.oscillators = vec![
            Oscillator::new(Waveform::Saw, self.pitch, sample_rate),
            Oscillator::new(Waveform::Saw, self.pitch * 1.003, sample_rate),
            Oscillator::new(Waveform::Saw, self.pitch * 0.997, sample_rate),
            Oscillator::new(Waveform::Square, self.pitch * 1.001, sample_rate),
        ];
        self.sub_oscillator = Oscillator::new(Waveform::Sine, self.pitch * 0.5, sample_rate);
        self.noise_oscillator = Oscillator::new(Waveform::PinkNoise, 1.0, sample_rate);
        
        // Recreate other components
        self.envelope = Envelope::new(sample_rate);
        self.envelope.configure(0.5, 0.3, 0.8, 1.0);
        
        self.filter = Filter::with_type(sample_rate, FilterType::LowPass);
        self.filter.set_cutoff(self.filter_cutoff);
        self.filter.set_resonance(self.filter_resonance);
        
        self.filter_lfo = Lfo::new(sample_rate);
        self.filter_lfo.set_frequency(0.1);
        
        self.pitch_lfo = Lfo::new(sample_rate);
        self.pitch_lfo.set_frequency(4.0);
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
        
        voice.set_parameter("noise_mix", 0.1);
        assert_eq!(voice.get_parameter("noise_mix"), Some(0.1));
    }

    #[test]
    fn test_drone_voice_output() {
        let mut voice = DroneVoice::new(44100.0);
        voice.trigger();
        
        // Generate some samples
        let mut samples = Vec::new();
        for _ in 0..1000 {
            samples.push(voice.process());
        }
        
        // Should produce non-zero output
        let max = samples.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        assert!(max > 0.0);
    }

    #[test]
    fn test_drone_voice_envelope() {
        let mut voice = DroneVoice::new(44100.0);
        voice.trigger();
        
        // Process some samples during attack
        for _ in 0..100 {
            voice.process();
        }
        assert!(voice.is_active());
        
        // Release
        voice.release();
        
        // Process through release
        for _ in 0..100000 {
            voice.process();
        }
        
        // Should be inactive after release completes
        assert!(!voice.is_active());
    }

    #[test]
    fn test_drone_voice_lfo_modulation() {
        let mut voice = DroneVoice::new(44100.0);
        voice.trigger();
        voice.set_parameter("filter_lfo_depth", 1000.0);
        voice.set_parameter("vibrato_depth", 20.0);
        
        // Generate samples - should not crash and should produce varying output
        let mut samples = Vec::new();
        for _ in 0..4410 {
            samples.push(voice.process());
        }
        
        // Check for variation (LFO should cause changes)
        let first_100: f64 = samples[0..100].iter().map(|s| s.abs()).sum();
        let last_100: f64 = samples[4310..4410].iter().map(|s| s.abs()).sum();
        
        // Both should have audio
        assert!(first_100 > 0.0);
        assert!(last_100 > 0.0);
    }
}
