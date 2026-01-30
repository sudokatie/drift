//! Basic oscillator implementation

use std::f64::consts::PI;

/// Waveform types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Triangle,
    Saw,
    Square,
}

/// A basic oscillator that generates waveforms
pub struct Oscillator {
    waveform: Waveform,
    phase: f64,
    frequency: f64,
    sample_rate: f64,
}

impl Oscillator {
    /// Create a new oscillator
    pub fn new(waveform: Waveform, frequency: f64, sample_rate: f64) -> Self {
        Self {
            waveform,
            phase: 0.0,
            frequency,
            sample_rate,
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
        };
        
        // Advance phase
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
}
