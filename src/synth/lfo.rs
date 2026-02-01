//! Low Frequency Oscillator for modulation
//!
//! Provides slow modulation for pitch, filter, amplitude, etc.

use std::f64::consts::PI;

/// LFO waveform shapes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LfoShape {
    Sine,
    Triangle,
    Saw,
    Square,
    SampleAndHold,
}

/// Low Frequency Oscillator
pub struct Lfo {
    shape: LfoShape,
    frequency: f64,
    phase: f64,
    sample_rate: f64,
    /// Depth of modulation (0.0 to 1.0)
    depth: f64,
    /// Last sample-and-hold value
    sh_value: f64,
    /// RNG state for S&H
    rng_state: u64,
}

impl Lfo {
    /// Create a new LFO
    pub fn new(sample_rate: f64) -> Self {
        Self {
            shape: LfoShape::Sine,
            frequency: 0.5, // 0.5 Hz default
            phase: 0.0,
            sample_rate,
            depth: 1.0,
            sh_value: 0.0,
            rng_state: 12345,
        }
    }
    
    /// Set LFO frequency in Hz
    pub fn set_frequency(&mut self, hz: f64) {
        self.frequency = hz.clamp(0.01, 100.0);
    }
    
    /// Get LFO frequency
    pub fn frequency(&self) -> f64 {
        self.frequency
    }
    
    /// Set modulation depth (0.0 to 1.0)
    pub fn set_depth(&mut self, depth: f64) {
        self.depth = depth.clamp(0.0, 1.0);
    }
    
    /// Get modulation depth
    pub fn depth(&self) -> f64 {
        self.depth
    }
    
    /// Set LFO shape
    pub fn set_shape(&mut self, shape: LfoShape) {
        self.shape = shape;
    }
    
    /// Reset phase
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
    
    /// Generate next sample (-1.0 to 1.0, scaled by depth)
    pub fn process(&mut self) -> f64 {
        let raw = match self.shape {
            LfoShape::Sine => (self.phase * 2.0 * PI).sin(),
            LfoShape::Triangle => {
                if self.phase < 0.25 {
                    4.0 * self.phase
                } else if self.phase < 0.75 {
                    2.0 - 4.0 * self.phase
                } else {
                    4.0 * self.phase - 4.0
                }
            }
            LfoShape::Saw => 2.0 * self.phase - 1.0,
            LfoShape::Square => if self.phase < 0.5 { 1.0 } else { -1.0 },
            LfoShape::SampleAndHold => {
                // Update value at phase wrap
                if self.phase < self.frequency / self.sample_rate {
                    self.sh_value = self.random();
                }
                self.sh_value
            }
        };
        
        // Advance phase
        self.phase += self.frequency / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        raw * self.depth
    }
    
    /// Generate unipolar output (0.0 to 1.0, scaled by depth)
    pub fn process_unipolar(&mut self) -> f64 {
        (self.process() + 1.0) * 0.5
    }
    
    /// Simple RNG for sample-and-hold
    fn random(&mut self) -> f64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        (x as f64 / u64::MAX as f64) * 2.0 - 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfo_creation() {
        let lfo = Lfo::new(44100.0);
        assert_eq!(lfo.frequency(), 0.5);
        assert_eq!(lfo.depth(), 1.0);
    }

    #[test]
    fn test_lfo_sine_range() {
        let mut lfo = Lfo::new(44100.0);
        lfo.set_shape(LfoShape::Sine);
        lfo.set_frequency(1.0);
        
        for _ in 0..44100 {
            let sample = lfo.process();
            assert!((-1.0..=1.0).contains(&sample));
        }
    }

    #[test]
    fn test_lfo_depth() {
        let mut lfo = Lfo::new(44100.0);
        lfo.set_depth(0.5);
        lfo.set_shape(LfoShape::Square);
        
        let sample = lfo.process();
        assert!(sample.abs() <= 0.5);
    }

    #[test]
    fn test_lfo_unipolar() {
        let mut lfo = Lfo::new(44100.0);
        lfo.set_shape(LfoShape::Sine);
        
        for _ in 0..1000 {
            let sample = lfo.process_unipolar();
            assert!((0.0..=1.0).contains(&sample));
        }
    }

    #[test]
    fn test_lfo_frequency_clamping() {
        let mut lfo = Lfo::new(44100.0);
        
        lfo.set_frequency(0.001);
        assert_eq!(lfo.frequency(), 0.01);
        
        lfo.set_frequency(200.0);
        assert_eq!(lfo.frequency(), 100.0);
    }
}
