//! Exponential mapper implementation
//!
//! Maps input values using exponential scaling, creating a logarithmic
//! perception curve. This is the inverse of LogarithmicMapper.
//!
//! Use cases:
//! - Volume faders (physical position -> perceived loudness)
//! - Any control where you want fine adjustment at low values

use super::Mapper;

/// Exponential mapper for inverse-perceptual scaling
/// 
/// Takes a linearly-scaled input and maps it to an output that
/// feels logarithmic to human perception. Small changes at low
/// input values produce large changes in output; large changes
/// at high input values produce small changes in output.
///
/// Uses the formula for normalized input t in [0, 1]:
///   output = out_min + (out_max - out_min) * (exp(k*t) - 1) / (exp(k) - 1)
///
/// where k controls the curve steepness (default: ln(out_max/out_min) to match LogarithmicMapper)
pub struct ExponentialMapper {
    name: String,
    in_min: f64,
    in_max: f64,
    out_min: f64,
    out_max: f64,
    curve_factor: f64,
    clamp: bool,
}

impl ExponentialMapper {
    /// Create a new exponential mapper
    /// 
    /// The curve factor is automatically calculated to create a true
    /// inverse of the logarithmic mapping curve.
    pub fn new(
        name: impl Into<String>,
        in_min: f64,
        in_max: f64,
        out_min: f64,
        out_max: f64,
    ) -> Self {
        // Calculate curve factor to match logarithmic inverse
        // Use ln(out_max/out_min) for consistency with LogarithmicMapper
        let out_min_safe = out_min.abs().max(0.001);
        let out_max_safe = out_max.abs().max(0.001);
        let curve_factor = (out_max_safe / out_min_safe).ln().max(1.0);
        
        Self {
            name: name.into(),
            in_min,
            in_max,
            out_min,
            out_max,
            curve_factor,
            clamp: true,
        }
    }
    
    /// Create with a custom curve factor
    /// 
    /// Higher values create steeper exponential curves.
    /// Typical values: 2.0 (mild) to 10.0 (steep)
    pub fn with_curve_factor(mut self, factor: f64) -> Self {
        self.curve_factor = factor.max(0.001);
        self
    }
    
    /// Set whether to clamp output to range
    pub fn with_clamp(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
}

impl Mapper for ExponentialMapper {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn map(&self, input: f64) -> f64 {
        // Normalize input to 0..1
        let in_range = self.in_max - self.in_min;
        let normalized = if in_range.abs() < f64::EPSILON {
            0.5
        } else {
            (input - self.in_min) / in_range
        };
        
        // Clamp normalized to 0..1 if enabled
        let normalized = if self.clamp {
            normalized.clamp(0.0, 1.0)
        } else {
            normalized
        };
        
        // Apply exponential scaling
        // output = out_min + (out_max - out_min) * (exp(k*t) - 1) / (exp(k) - 1)
        // where k = curve_factor, t = normalized input
        let k = self.curve_factor;
        let exp_k = k.exp();
        let exp_kt = (k * normalized).exp();
        
        let scaled = if (exp_k - 1.0).abs() < f64::EPSILON {
            // Degenerate case: k very small, use linear
            normalized
        } else {
            (exp_kt - 1.0) / (exp_k - 1.0)
        };
        
        let out_range = self.out_max - self.out_min;
        let output = self.out_min + out_range * scaled;
        
        // Final clamp if enabled
        if self.clamp {
            output.clamp(self.out_min.min(self.out_max), self.out_min.max(self.out_max))
        } else {
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_mapper_basic() {
        // Map 0-100 to 0-1000
        let mapper = ExponentialMapper::new("test", 0.0, 100.0, 0.0, 1000.0);
        
        // At 0%, should be at minimum
        let result = mapper.map(0.0);
        assert!((result - 0.0).abs() < 0.01, "Expected 0, got {}", result);
        
        // At 100%, should be at maximum
        let result = mapper.map(100.0);
        assert!((result - 1000.0).abs() < 1.0, "Expected 1000, got {}", result);
    }

    #[test]
    fn test_exponential_curve_shape() {
        // With exponential mapping, 50% input should give LESS than 50% output
        // (the curve is "slow to start, fast at end")
        let mapper = ExponentialMapper::new("test", 0.0, 100.0, 0.0, 1000.0)
            .with_curve_factor(3.0);
        
        let at_50 = mapper.map(50.0);
        // With k=3, at t=0.5: (exp(1.5) - 1) / (exp(3) - 1) = 3.48/19.09 = 0.182
        // So output should be about 182, much less than 500 (linear midpoint)
        assert!(at_50 < 500.0, "Expected < 500, got {}", at_50);
    }

    #[test]
    fn test_exponential_mapper_clamped() {
        let mapper = ExponentialMapper::new("test", 0.0, 100.0, 10.0, 1000.0);
        
        // Values outside range should be clamped
        let below = mapper.map(-50.0);
        assert_eq!(below, 10.0);
        
        let above = mapper.map(150.0);
        assert_eq!(above, 1000.0);
    }

    #[test]
    fn test_exponential_vs_linear() {
        // Exponential should differ from linear at midpoint
        let exp_mapper = ExponentialMapper::new("exp", 0.0, 100.0, 0.0, 1000.0)
            .with_curve_factor(4.0);
        
        let exp_50 = exp_mapper.map(50.0);
        let linear_50 = 500.0;
        
        // Exponential should be significantly less than linear at midpoint
        assert!(exp_50 < linear_50 * 0.5, 
            "Expected exponential midpoint {} to be much less than linear midpoint {}", 
            exp_50, linear_50);
    }

    #[test]
    fn test_exponential_endpoints() {
        // Regardless of curve factor, endpoints should match
        let mapper = ExponentialMapper::new("test", 0.0, 1.0, 100.0, 900.0)
            .with_curve_factor(5.0);
        
        assert!((mapper.map(0.0) - 100.0).abs() < 0.01);
        assert!((mapper.map(1.0) - 900.0).abs() < 0.1);
    }

    #[test]
    fn test_exponential_inverted_range() {
        // Inverted range (high to low)
        let mapper = ExponentialMapper::new("test", 0.0, 100.0, 1000.0, 100.0);
        
        let at_0 = mapper.map(0.0);
        let at_100 = mapper.map(100.0);
        
        assert!((at_0 - 1000.0).abs() < 0.1);
        assert!((at_100 - 100.0).abs() < 0.1);
    }
}
