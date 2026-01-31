//! Logarithmic mapper implementation
//!
//! Maps input values using logarithmic scaling, useful for
//! frequency and volume mapping where human perception is logarithmic.

use super::Mapper;

/// Logarithmic mapper for perceptual scaling
/// 
/// Uses the formula: output = out_min * (out_max/out_min)^((input - in_min)/(in_max - in_min))
/// 
/// This creates an exponential curve that sounds linear to human perception
/// (e.g., for frequency: doubling frequency = one octave, regardless of starting point)
pub struct LogarithmicMapper {
    name: String,
    in_min: f64,
    in_max: f64,
    out_min: f64,
    out_max: f64,
    clamp: bool,
}

impl LogarithmicMapper {
    /// Create a new logarithmic mapper
    /// 
    /// Note: out_min must be > 0 for logarithmic scaling to work
    pub fn new(
        name: impl Into<String>,
        in_min: f64,
        in_max: f64,
        out_min: f64,
        out_max: f64,
    ) -> Self {
        Self {
            name: name.into(),
            in_min,
            in_max,
            // Ensure out_min is positive for log calculation
            out_min: out_min.max(0.001),
            out_max: out_max.max(0.001),
            clamp: true,
        }
    }
    
    /// Set whether to clamp output to range
    pub fn with_clamp(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
}

impl Mapper for LogarithmicMapper {
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
        
        // Apply logarithmic scaling
        // output = out_min * (out_max/out_min)^normalized
        let ratio = self.out_max / self.out_min;
        let output = self.out_min * ratio.powf(normalized);
        
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
    fn test_logarithmic_mapper_basic() {
        // Map 0-100 to 20-20000 Hz (frequency range)
        let mapper = LogarithmicMapper::new("freq", 0.0, 100.0, 20.0, 20000.0);
        
        // At 0%, should be at minimum
        let result = mapper.map(0.0);
        assert!((result - 20.0).abs() < 0.01, "Expected 20, got {}", result);
        
        // At 100%, should be at maximum
        let result = mapper.map(100.0);
        assert!((result - 20000.0).abs() < 1.0, "Expected 20000, got {}", result);
        
        // At 50%, should be geometric mean (sqrt(20 * 20000) = 632.5)
        let result = mapper.map(50.0);
        let expected = (20.0 * 20000.0_f64).sqrt();
        assert!((result - expected).abs() < 1.0, "Expected {}, got {}", expected, result);
    }

    #[test]
    fn test_logarithmic_mapper_octaves() {
        // Map 0-3 to 100-800 Hz (3 octaves)
        let mapper = LogarithmicMapper::new("octave", 0.0, 3.0, 100.0, 800.0);
        
        // Each unit should double the frequency (one octave)
        let f0 = mapper.map(0.0);
        let f1 = mapper.map(1.0);
        let f2 = mapper.map(2.0);
        let f3 = mapper.map(3.0);
        
        assert!((f0 - 100.0).abs() < 0.1);
        assert!((f1 - 200.0).abs() < 0.1, "Expected 200, got {}", f1);
        assert!((f2 - 400.0).abs() < 0.1, "Expected 400, got {}", f2);
        assert!((f3 - 800.0).abs() < 0.1);
    }

    #[test]
    fn test_logarithmic_mapper_clamped() {
        let mapper = LogarithmicMapper::new("test", 0.0, 100.0, 10.0, 1000.0);
        
        // Values outside range should be clamped
        let below = mapper.map(-50.0);
        assert_eq!(below, 10.0);
        
        let above = mapper.map(150.0);
        assert_eq!(above, 1000.0);
    }

    #[test]
    fn test_logarithmic_mapper_small_range() {
        // Even small ranges should work
        let mapper = LogarithmicMapper::new("test", 0.0, 1.0, 1.0, 2.0);
        
        let result = mapper.map(0.5);
        let expected = 2.0_f64.sqrt(); // ~1.414
        assert!((result - expected).abs() < 0.01);
    }

    #[test]
    fn test_logarithmic_mapper_inverted() {
        // Inverted range (high to low)
        let mapper = LogarithmicMapper::new("test", 0.0, 100.0, 1000.0, 100.0);
        
        let at_0 = mapper.map(0.0);
        let at_100 = mapper.map(100.0);
        
        assert!((at_0 - 1000.0).abs() < 0.1);
        assert!((at_100 - 100.0).abs() < 0.1);
    }
}
