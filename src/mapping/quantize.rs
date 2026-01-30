//! Quantize mapper for snapping to musical scales

use super::Mapper;

/// Musical scale definition (intervals in semitones from root)
#[derive(Debug, Clone)]
pub struct Scale {
    name: String,
    intervals: Vec<u8>,
}

impl Scale {
    /// Create a new scale
    pub fn new(name: &str, intervals: Vec<u8>) -> Self {
        Self {
            name: name.to_string(),
            intervals,
        }
    }
    
    /// Minor pentatonic scale (root, m3, P4, P5, m7)
    pub fn minor_pentatonic() -> Self {
        Self::new("minor_pentatonic", vec![0, 3, 5, 7, 10])
    }
    
    /// Major pentatonic scale (root, M2, M3, P5, M6)
    pub fn major_pentatonic() -> Self {
        Self::new("major_pentatonic", vec![0, 2, 4, 7, 9])
    }
    
    /// Natural minor scale
    pub fn minor() -> Self {
        Self::new("minor", vec![0, 2, 3, 5, 7, 8, 10])
    }
    
    /// Major scale
    pub fn major() -> Self {
        Self::new("major", vec![0, 2, 4, 5, 7, 9, 11])
    }
    
    /// Dorian mode
    pub fn dorian() -> Self {
        Self::new("dorian", vec![0, 2, 3, 5, 7, 9, 10])
    }
    
    /// Whole tone scale
    pub fn whole_tone() -> Self {
        Self::new("whole_tone", vec![0, 2, 4, 6, 8, 10])
    }
    
    /// Get scale by name
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "pentatonic" | "minor_pentatonic" | "minorpentatonic" => Some(Self::minor_pentatonic()),
            "major_pentatonic" | "majorpentatonic" => Some(Self::major_pentatonic()),
            "minor" | "natural_minor" => Some(Self::minor()),
            "major" => Some(Self::major()),
            "dorian" => Some(Self::dorian()),
            "whole_tone" | "wholetone" => Some(Self::whole_tone()),
            _ => None,
        }
    }
    
    /// Get the name of this scale
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the intervals
    pub fn intervals(&self) -> &[u8] {
        &self.intervals
    }
}

/// A mapper that quantizes frequencies to a musical scale
pub struct QuantizeMapper {
    name: String,
    root_hz: f64,
    scale: Scale,
}

impl QuantizeMapper {
    /// Create a new quantize mapper
    /// 
    /// # Arguments
    /// * `name` - Name for this mapper
    /// * `root_hz` - Root frequency in Hz (e.g., 440.0 for A4)
    /// * `scale` - The scale to quantize to
    pub fn new(name: &str, root_hz: f64, scale: Scale) -> Self {
        Self {
            name: name.to_string(),
            root_hz,
            scale,
        }
    }
    
    /// Convert frequency to semitones from root
    fn hz_to_semitones(&self, hz: f64) -> f64 {
        12.0 * (hz / self.root_hz).log2()
    }
    
    /// Convert semitones from root to frequency
    fn semitones_to_hz(&self, semitones: f64) -> f64 {
        self.root_hz * 2.0_f64.powf(semitones / 12.0)
    }
    
    /// Quantize a semitone value to the nearest scale degree
    fn quantize_semitones(&self, semitones: f64) -> f64 {
        // Normalize to octave (0-12 range)
        let octave = (semitones / 12.0).floor();
        let normalized = semitones - (octave * 12.0);
        
        // Handle negative values
        let (octave, normalized) = if normalized < 0.0 {
            (octave - 1.0, normalized + 12.0)
        } else {
            (octave, normalized)
        };
        
        // Find nearest scale degree
        let mut nearest = 0.0;
        let mut min_dist = f64::MAX;
        
        for &interval in self.scale.intervals() {
            let dist = (normalized - interval as f64).abs();
            if dist < min_dist {
                min_dist = dist;
                nearest = interval as f64;
            }
            // Also check wrapping to next octave
            let dist_wrap = (normalized - (interval as f64 + 12.0)).abs();
            if dist_wrap < min_dist {
                min_dist = dist_wrap;
                nearest = interval as f64 + 12.0;
            }
        }
        
        // Return quantized semitones
        (octave * 12.0) + nearest
    }
}

impl Mapper for QuantizeMapper {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn map(&self, input: f64) -> f64 {
        // Input is frequency in Hz
        if input <= 0.0 {
            return input;
        }
        
        let semitones = self.hz_to_semitones(input);
        let quantized = self.quantize_semitones(semitones);
        self.semitones_to_hz(quantized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_creation() {
        let scale = Scale::minor_pentatonic();
        assert_eq!(scale.name(), "minor_pentatonic");
        assert_eq!(scale.intervals(), &[0, 3, 5, 7, 10]);
    }

    #[test]
    fn test_scale_from_name() {
        assert!(Scale::from_name("minor_pentatonic").is_some());
        assert!(Scale::from_name("major").is_some());
        assert!(Scale::from_name("dorian").is_some());
        assert!(Scale::from_name("unknown").is_none());
    }

    #[test]
    fn test_quantize_to_root() {
        let mapper = QuantizeMapper::new(
            "test",
            440.0, // A4
            Scale::minor_pentatonic(),
        );
        
        // Input exactly at root should stay at root
        let result = mapper.map(440.0);
        assert!((result - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_quantize_to_scale_degree() {
        let mapper = QuantizeMapper::new(
            "test",
            440.0, // A4
            Scale::minor_pentatonic(), // A, C, D, E, G
        );
        
        // A4 = 440 Hz
        // C5 = 523.25 Hz (3 semitones up)
        // D5 = 587.33 Hz (5 semitones up)
        
        // 500 Hz (~2.2 semitones) should snap to C5 (523.25 Hz, 3 semitones)
        let result = mapper.map(500.0);
        assert!((result - 523.25).abs() < 1.0, "Expected ~523 Hz, got {}", result);
        
        // 560 Hz (~4.2 semitones) should snap to D5 (587.33 Hz, 5 semitones)
        let result = mapper.map(560.0);
        assert!((result - 587.33).abs() < 1.0, "Expected ~587 Hz, got {}", result);
    }

    #[test]
    fn test_quantize_octave_wrapping() {
        let mapper = QuantizeMapper::new(
            "test",
            440.0,
            Scale::minor_pentatonic(),
        );
        
        // 880 Hz = A5 (one octave up) should stay at 880
        let result = mapper.map(880.0);
        assert!((result - 880.0).abs() < 0.01);
        
        // 220 Hz = A3 (one octave down) should stay at 220
        let result = mapper.map(220.0);
        assert!((result - 220.0).abs() < 0.01);
    }

    #[test]
    fn test_quantize_major_scale() {
        let mapper = QuantizeMapper::new(
            "test",
            261.63, // C4 (middle C)
            Scale::major(), // C, D, E, F, G, A, B
        );
        
        // Input at C4 should stay at C4
        let result = mapper.map(261.63);
        assert!((result - 261.63).abs() < 0.1);
        
        // 280 Hz is between C4 (261.63) and D4 (293.66)
        // It should snap to D4
        let result = mapper.map(280.0);
        assert!((result - 293.66).abs() < 1.0, "Expected ~294 Hz, got {}", result);
    }

    #[test]
    fn test_quantize_handles_zero() {
        let mapper = QuantizeMapper::new(
            "test",
            440.0,
            Scale::minor_pentatonic(),
        );
        
        // Zero frequency should return zero (no crash)
        let result = mapper.map(0.0);
        assert_eq!(result, 0.0);
    }
}
