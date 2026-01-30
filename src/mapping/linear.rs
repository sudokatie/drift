//! Linear mapper implementation

use super::Mapper;

/// Linear interpolation mapper
pub struct LinearMapper {
    name: String,
    in_min: f64,
    in_max: f64,
    out_min: f64,
    out_max: f64,
    clamp: bool,
}

impl LinearMapper {
    /// Create a new linear mapper
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
            out_min,
            out_max,
            clamp: true,
        }
    }
    
    /// Set whether to clamp output to range
    pub fn with_clamp(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
}

impl Mapper for LinearMapper {
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
        
        // Scale to output range
        let out_range = self.out_max - self.out_min;
        let output = self.out_min + normalized * out_range;
        
        // Clamp if enabled
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
    fn test_linear_mapper_basic() {
        let mapper = LinearMapper::new("test", 0.0, 100.0, 0.0, 1.0);
        
        assert_eq!(mapper.map(0.0), 0.0);
        assert_eq!(mapper.map(50.0), 0.5);
        assert_eq!(mapper.map(100.0), 1.0);
    }

    #[test]
    fn test_linear_mapper_inverted() {
        let mapper = LinearMapper::new("test", 0.0, 100.0, 1.0, 0.0);
        
        assert_eq!(mapper.map(0.0), 1.0);
        assert_eq!(mapper.map(50.0), 0.5);
        assert_eq!(mapper.map(100.0), 0.0);
    }

    #[test]
    fn test_linear_mapper_clamped() {
        let mapper = LinearMapper::new("test", 0.0, 100.0, 0.0, 1.0);
        
        // Values outside range should be clamped
        assert_eq!(mapper.map(-50.0), 0.0);
        assert_eq!(mapper.map(150.0), 1.0);
    }

    #[test]
    fn test_linear_mapper_unclamped() {
        let mapper = LinearMapper::new("test", 0.0, 100.0, 0.0, 1.0)
            .with_clamp(false);
        
        // Values outside range should extrapolate
        assert_eq!(mapper.map(-50.0), -0.5);
        assert_eq!(mapper.map(150.0), 1.5);
    }

    #[test]
    fn test_linear_mapper_temperature_to_pitch() {
        // Temperature -20..40 -> Pitch 100..400 Hz
        let mapper = LinearMapper::new("temp_to_pitch", -20.0, 40.0, 100.0, 400.0);
        
        assert_eq!(mapper.map(-20.0), 100.0);  // Cold = low pitch
        assert_eq!(mapper.map(10.0), 250.0);   // Mid = mid pitch
        assert_eq!(mapper.map(40.0), 400.0);   // Hot = high pitch
    }
}
