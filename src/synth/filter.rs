//! Biquad filter implementation
//!
//! Digital biquad filter for audio processing.

use std::f64::consts::PI;

/// Filter type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
}

/// Biquad filter coefficients
#[derive(Debug, Clone, Copy)]
struct Coefficients {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
}

impl Default for Coefficients {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

/// Biquad filter for audio processing
pub struct Filter {
    filter_type: FilterType,
    sample_rate: f64,
    cutoff: f64,
    resonance: f64, // Q factor
    
    coeffs: Coefficients,
    
    // Filter state (Direct Form II transposed)
    z1: f64,
    z2: f64,
}

impl Filter {
    /// Create a new low-pass filter
    pub fn new(sample_rate: f64) -> Self {
        let mut filter = Self {
            filter_type: FilterType::LowPass,
            sample_rate,
            cutoff: 1000.0,
            resonance: 0.707, // Butterworth Q
            coeffs: Coefficients::default(),
            z1: 0.0,
            z2: 0.0,
        };
        filter.calculate_coefficients();
        filter
    }
    
    /// Create a filter with specific type
    pub fn with_type(sample_rate: f64, filter_type: FilterType) -> Self {
        let mut filter = Self {
            filter_type,
            sample_rate,
            cutoff: 1000.0,
            resonance: 0.707,
            coeffs: Coefficients::default(),
            z1: 0.0,
            z2: 0.0,
        };
        filter.calculate_coefficients();
        filter
    }
    
    /// Set cutoff frequency in Hz
    pub fn set_cutoff(&mut self, hz: f64) {
        // Clamp to valid range (20 Hz to Nyquist - margin)
        self.cutoff = hz.clamp(20.0, self.sample_rate * 0.45);
        self.calculate_coefficients();
    }
    
    /// Get cutoff frequency
    pub fn cutoff(&self) -> f64 {
        self.cutoff
    }
    
    /// Set resonance (Q factor)
    /// Higher values = more resonance at cutoff
    /// 0.707 = Butterworth (flat response)
    /// > 1.0 = resonant peak
    pub fn set_resonance(&mut self, q: f64) {
        // Clamp Q to prevent instability
        self.resonance = q.clamp(0.1, 20.0);
        self.calculate_coefficients();
    }
    
    /// Get resonance
    pub fn resonance(&self) -> f64 {
        self.resonance
    }
    
    /// Set filter type
    pub fn set_type(&mut self, filter_type: FilterType) {
        self.filter_type = filter_type;
        self.calculate_coefficients();
    }
    
    /// Reset filter state (clear history)
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
    
    /// Calculate biquad coefficients based on current parameters
    fn calculate_coefficients(&mut self) {
        let omega = 2.0 * PI * self.cutoff / self.sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * self.resonance);
        
        let (b0, b1, b2, a0, a1, a2) = match self.filter_type {
            FilterType::LowPass => {
                let b0 = (1.0 - cos_omega) / 2.0;
                let b1 = 1.0 - cos_omega;
                let b2 = (1.0 - cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b0 = (1.0 + cos_omega) / 2.0;
                let b1 = -(1.0 + cos_omega);
                let b2 = (1.0 + cos_omega) / 2.0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };
        
        // Normalize by a0
        self.coeffs = Coefficients {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        };
    }
    
    /// Process a single sample through the filter
    pub fn process(&mut self, input: f64) -> f64 {
        // Direct Form II Transposed
        let output = self.coeffs.b0 * input + self.z1;
        
        self.z1 = self.coeffs.b1 * input - self.coeffs.a1 * output + self.z2;
        self.z2 = self.coeffs.b2 * input - self.coeffs.a2 * output;
        
        output
    }
    
    /// Process a buffer of samples in place
    pub fn process_buffer(&mut self, buffer: &mut [f64]) {
        for sample in buffer.iter_mut() {
            *sample = self.process(*sample);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_creation() {
        let filter = Filter::new(44100.0);
        assert_eq!(filter.cutoff(), 1000.0);
        assert!((filter.resonance() - 0.707).abs() < 0.001);
    }

    #[test]
    fn test_filter_cutoff_clamping() {
        let mut filter = Filter::new(44100.0);
        
        // Too low
        filter.set_cutoff(5.0);
        assert_eq!(filter.cutoff(), 20.0);
        
        // Too high (above Nyquist)
        filter.set_cutoff(25000.0);
        assert!(filter.cutoff() < 44100.0 * 0.5);
    }

    #[test]
    fn test_filter_resonance_clamping() {
        let mut filter = Filter::new(44100.0);
        
        filter.set_resonance(0.01);
        assert_eq!(filter.resonance(), 0.1);
        
        filter.set_resonance(100.0);
        assert_eq!(filter.resonance(), 20.0);
    }

    #[test]
    fn test_lowpass_attenuates_high_frequencies() {
        let mut filter = Filter::new(44100.0);
        filter.set_cutoff(100.0); // Very low cutoff
        
        // Generate high frequency signal (5000 Hz)
        let freq = 5000.0;
        let mut max_input = 0.0f64;
        let mut max_output = 0.0f64;
        
        for i in 0..1000 {
            let t = i as f64 / 44100.0;
            let input = (2.0 * PI * freq * t).sin();
            let output = filter.process(input);
            
            max_input = max_input.max(input.abs());
            max_output = max_output.max(output.abs());
        }
        
        // High frequency should be significantly attenuated
        assert!(max_output < max_input * 0.1, 
            "Expected attenuation, got output={} input={}", max_output, max_input);
    }

    #[test]
    fn test_lowpass_passes_low_frequencies() {
        let mut filter = Filter::new(44100.0);
        filter.set_cutoff(5000.0);
        
        // Generate low frequency signal (100 Hz)
        let freq = 100.0;
        let mut sum_input_sq = 0.0;
        let mut sum_output_sq = 0.0;
        
        // Process enough samples to reach steady state
        for i in 0..4410 {
            let t = i as f64 / 44100.0;
            let input = (2.0 * PI * freq * t).sin();
            let output = filter.process(input);
            
            // Only measure after settling (skip first 100 samples)
            if i > 100 {
                sum_input_sq += input * input;
                sum_output_sq += output * output;
            }
        }
        
        let rms_input = (sum_input_sq / 4310.0).sqrt();
        let rms_output = (sum_output_sq / 4310.0).sqrt();
        
        // Low frequency should pass through mostly unchanged
        let ratio = rms_output / rms_input;
        assert!(ratio > 0.9, "Expected passthrough, got ratio={}", ratio);
    }

    #[test]
    fn test_highpass_filter() {
        let mut filter = Filter::with_type(44100.0, FilterType::HighPass);
        filter.set_cutoff(1000.0);
        
        // Low frequency (100 Hz) should be attenuated
        let freq = 100.0;
        let mut max_output = 0.0f64;
        
        for i in 0..2000 {
            let t = i as f64 / 44100.0;
            let input = (2.0 * PI * freq * t).sin();
            let output = filter.process(input);
            
            if i > 500 {
                max_output = max_output.max(output.abs());
            }
        }
        
        // Low frequency should be attenuated
        assert!(max_output < 0.5, "Expected attenuation, got {}", max_output);
    }

    #[test]
    fn test_filter_reset() {
        let mut filter = Filter::new(44100.0);
        
        // Process some samples
        for _ in 0..100 {
            filter.process(1.0);
        }
        
        // Reset
        filter.reset();
        
        // First output after reset should be just the input scaled
        let output = filter.process(0.0);
        assert!(output.abs() < 0.001, "Expected near-zero after reset, got {}", output);
    }

    #[test]
    fn test_process_buffer() {
        let mut filter = Filter::new(44100.0);
        filter.set_cutoff(100.0);
        
        // Generate high frequency buffer
        let freq = 5000.0;
        let mut buffer: Vec<f64> = (0..1000)
            .map(|i| {
                let t = i as f64 / 44100.0;
                (2.0 * PI * freq * t).sin()
            })
            .collect();
        
        filter.process_buffer(&mut buffer);
        
        // Check that high frequencies are attenuated
        let max = buffer.iter().fold(0.0f64, |a, &b| a.max(b.abs()));
        assert!(max < 0.2);
    }
}
