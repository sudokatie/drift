//! Basic oscillator implementation

use std::f64::consts::PI;

/// Waveform types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Square,
    /// White noise (uniform random)
    WhiteNoise,
    /// Pink noise (1/f, more natural sounding)
    PinkNoise,
    /// Brown noise (1/f^2, deep rumble)
    BrownNoise,
}

/// A basic oscillator that generates waveforms
pub struct Oscillator {
    waveform: Waveform,
    phase: f64,
    frequency: f64,
    sample_rate: f64,
    /// State for pink noise (Paul Kellet's algorithm)
    pink_state: [f64; 7],
    /// State for brown noise (integration of white noise)
    brown_state: f64,
    /// Simple RNG state (xorshift)
    rng_state: u64,
}

impl Oscillator {
    /// Create a new oscillator
    pub fn new(waveform: Waveform, frequency: f64, sample_rate: f64) -> Self {
        Self {
            waveform,
            phase: 0.0,
            frequency,
            sample_rate,
            pink_state: [0.0; 7],
            brown_state: 0.0,
            // Initialize RNG with a non-zero seed based on frequency
            rng_state: ((frequency * 1000.0) as u64).max(1),
        }
    }
    
    /// Set the frequency
    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
    }
    
    /// Get the current frequency
    pub fn frequency(&self) -> f64 {
        self.frequency
    }
    
    /// Set the waveform
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }
    
    /// Reset the phase
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
    
    /// Generate the next sample
    pub fn generate(&mut self) -> f64 {
        let sample = match self.waveform {
            Waveform::Sine => self.sine(),
            Waveform::Triangle => self.triangle(),
            Waveform::Saw => self.saw(),
            Waveform::Square => self.square(),
            Waveform::WhiteNoise => self.white_noise(),
            Waveform::PinkNoise => self.pink_noise(),
            Waveform::BrownNoise => self.brown_noise(),
        };
        
        // Advance phase (not used for noise but keeps state consistent)
        self.phase += self.frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        sample
    }
    
    fn sine(&self) -> f64 {
        (self.phase * 2.0 * PI).sin()
    }
    
    fn triangle(&self) -> f64 {
        let p = self.phase;
        if p < 0.25 {
            4.0 * p
        } else if p < 0.75 {
            2.0 - 4.0 * p
        } else {
            4.0 * p - 4.0
        }
    }
    
    fn saw(&self) -> f64 {
        2.0 * self.phase - 1.0
    }
    
    fn square(&self) -> f64 {
        if self.phase < 0.5 { 1.0 } else { -1.0 }
    }
    
    /// Xorshift RNG for noise generation
    fn random(&mut self) -> f64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        // Convert to -1.0..1.0 range
        (x as f64 / u64::MAX as f64) * 2.0 - 1.0
    }
    
    /// White noise - uniform random samples
    fn white_noise(&mut self) -> f64 {
        self.random()
    }
    
    /// Pink noise using Paul Kellet's economy method
    /// Attempt to generate 1/f noise with minimal computation
    fn pink_noise(&mut self) -> f64 {
        let white = self.random();
        
        // Paul Kellet's economy pink noise filter
        self.pink_state[0] = 0.99886 * self.pink_state[0] + white * 0.0555179;
        self.pink_state[1] = 0.99332 * self.pink_state[1] + white * 0.0750759;
        self.pink_state[2] = 0.96900 * self.pink_state[2] + white * 0.1538520;
        self.pink_state[3] = 0.86650 * self.pink_state[3] + white * 0.3104856;
        self.pink_state[4] = 0.55000 * self.pink_state[4] + white * 0.5329522;
        self.pink_state[5] = -0.7616 * self.pink_state[5] - white * 0.0168980;
        
        let pink = self.pink_state[0] + self.pink_state[1] + self.pink_state[2] 
            + self.pink_state[3] + self.pink_state[4] + self.pink_state[5] 
            + self.pink_state[6] + white * 0.5362;
        
        self.pink_state[6] = white * 0.115926;
        
        // Normalize to roughly -1..1 range
        pink * 0.11
    }
    
    /// Brown noise - integration of white noise (1/f^2)
    fn brown_noise(&mut self) -> f64 {
        let white = self.random();
        
        // Leaky integrator
        self.brown_state = (self.brown_state + white * 0.02) * 0.999;
        
        // Clamp to prevent runaway
        self.brown_state = self.brown_state.clamp(-1.0, 1.0);
        
        self.brown_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_oscillator() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0);
        
        // First sample should be 0 (sin(0))
        let sample = osc.generate();
        assert!((sample - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_square_oscillator() {
        let mut osc = Oscillator::new(Waveform::Square, 1.0, 4.0);
        
        // 4 samples per cycle at 1 Hz, 4 Hz sample rate
        assert_eq!(osc.generate(), 1.0);  // phase 0.0
        assert_eq!(osc.generate(), 1.0);  // phase 0.25
        assert_eq!(osc.generate(), -1.0); // phase 0.5
        assert_eq!(osc.generate(), -1.0); // phase 0.75
    }

    #[test]
    fn test_saw_oscillator() {
        let mut osc = Oscillator::new(Waveform::Saw, 1.0, 4.0);
        
        // Saw goes from -1 to 1 linearly
        assert_eq!(osc.generate(), -1.0); // phase 0.0
        assert_eq!(osc.generate(), -0.5); // phase 0.25
        assert_eq!(osc.generate(), 0.0);  // phase 0.5
        assert_eq!(osc.generate(), 0.5);  // phase 0.75
    }

    #[test]
    fn test_frequency_change() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0);
        assert_eq!(osc.frequency(), 440.0);
        
        osc.set_frequency(880.0);
        assert_eq!(osc.frequency(), 880.0);
    }

    #[test]
    fn test_white_noise() {
        let mut osc = Oscillator::new(Waveform::WhiteNoise, 440.0, 44100.0);
        
        // Generate samples and check they're in range
        let mut sum = 0.0;
        for _ in 0..1000 {
            let sample = osc.generate();
            assert!((-1.0..=1.0).contains(&sample), "Sample out of range: {}", sample);
            sum += sample;
        }
        
        // Mean should be close to 0 for uniform noise
        let mean = sum / 1000.0;
        assert!(mean.abs() < 0.1, "Mean too far from 0: {}", mean);
    }

    #[test]
    fn test_pink_noise() {
        let mut osc = Oscillator::new(Waveform::PinkNoise, 440.0, 44100.0);
        
        // Generate samples and check they're roughly in range
        for _ in 0..1000 {
            let sample = osc.generate();
            assert!((-2.0..=2.0).contains(&sample), "Sample out of range: {}", sample);
        }
    }

    #[test]
    fn test_brown_noise() {
        let mut osc = Oscillator::new(Waveform::BrownNoise, 440.0, 44100.0);
        
        // Generate samples and check they're in range
        for _ in 0..1000 {
            let sample = osc.generate();
            assert!((-1.0..=1.0).contains(&sample), "Sample out of range: {}", sample);
        }
    }
}
