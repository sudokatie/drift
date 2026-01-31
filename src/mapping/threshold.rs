//! Threshold mapper implementation
//!
//! Triggers discrete events when input crosses a threshold value.
//! Useful for converting continuous data into note triggers.

use super::Mapper;

/// Threshold crossing direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThresholdDirection {
    /// Trigger when value rises above threshold
    Rising,
    /// Trigger when value falls below threshold
    Falling,
    /// Trigger on either direction
    Both,
}

/// Threshold mapper for triggering events
/// 
/// Outputs a trigger value (1.0) when input crosses the threshold,
/// and a rest value (0.0) otherwise. Can detect rising, falling, or both edges.
/// 
/// Note: This is a stateless level-based detector. For edge detection
/// (detecting the moment of crossing), use EdgeThresholdMapper.
pub struct ThresholdMapper {
    name: String,
    threshold: f64,
    direction: ThresholdDirection,
    /// Value to output when triggered
    trigger_value: f64,
    /// Value to output when not triggered  
    rest_value: f64,
    /// Hysteresis amount (prevents rapid toggling at threshold)
    hysteresis: f64,
}

impl ThresholdMapper {
    /// Create a new threshold mapper
    pub fn new(name: impl Into<String>, threshold: f64) -> Self {
        Self {
            name: name.into(),
            threshold,
            direction: ThresholdDirection::Rising,
            trigger_value: 1.0,
            rest_value: 0.0,
            hysteresis: 0.0,
        }
    }
    
    /// Set the threshold crossing direction
    pub fn with_direction(mut self, direction: ThresholdDirection) -> Self {
        self.direction = direction;
        self
    }
    
    /// Set the value to output when triggered
    pub fn with_trigger_value(mut self, value: f64) -> Self {
        self.trigger_value = value;
        self
    }
    
    /// Set the value to output when not triggered
    pub fn with_rest_value(mut self, value: f64) -> Self {
        self.rest_value = value;
        self
    }
    
    /// Set hysteresis amount to prevent rapid toggling
    /// 
    /// When crossing up, threshold becomes (threshold + hysteresis/2)
    /// When crossing down, threshold becomes (threshold - hysteresis/2)
    pub fn with_hysteresis(mut self, hysteresis: f64) -> Self {
        self.hysteresis = hysteresis.abs();
        self
    }
}

impl Mapper for ThresholdMapper {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn map(&self, input: f64) -> f64 {
        // For stateless Mapper trait, we use a simplified approach:
        // Return trigger_value if above threshold, rest_value if below
        // Note: Edge detection requires mutable state, so this is level-based
        
        let upper_threshold = self.threshold + self.hysteresis / 2.0;
        let lower_threshold = self.threshold - self.hysteresis / 2.0;
        
        match self.direction {
            ThresholdDirection::Rising => {
                if input >= upper_threshold {
                    self.trigger_value
                } else {
                    self.rest_value
                }
            }
            ThresholdDirection::Falling => {
                if input <= lower_threshold {
                    self.trigger_value
                } else {
                    self.rest_value
                }
            }
            ThresholdDirection::Both => {
                if input >= upper_threshold || input <= lower_threshold {
                    self.trigger_value
                } else {
                    self.rest_value
                }
            }
        }
    }
}

/// Edge-detecting threshold mapper (stateful)
/// 
/// Unlike ThresholdMapper which uses level detection, this detects
/// actual crossings and outputs a trigger only at the moment of crossing.
pub struct EdgeThresholdMapper {
    threshold: f64,
    direction: ThresholdDirection,
    trigger_value: f64,
    rest_value: f64,
    previous: f64,
    initialized: bool,
    hysteresis: f64,
}

impl EdgeThresholdMapper {
    /// Create a new edge-detecting threshold mapper
    pub fn new(_name: impl Into<String>, threshold: f64) -> Self {
        Self {
            threshold,
            direction: ThresholdDirection::Rising,
            trigger_value: 1.0,
            rest_value: 0.0,
            previous: f64::NEG_INFINITY,
            initialized: false,
            hysteresis: 0.0,
        }
    }
    
    /// Set the threshold crossing direction
    pub fn with_direction(mut self, direction: ThresholdDirection) -> Self {
        self.direction = direction;
        self
    }
    
    /// Set the value to output when triggered
    pub fn with_trigger_value(mut self, value: f64) -> Self {
        self.trigger_value = value;
        self
    }
    
    /// Set hysteresis
    pub fn with_hysteresis(mut self, hysteresis: f64) -> Self {
        self.hysteresis = hysteresis.abs();
        self
    }
    
    /// Process a value and return trigger or rest value
    pub fn process(&mut self, input: f64) -> f64 {
        if !self.initialized {
            self.previous = input;
            self.initialized = true;
            return self.rest_value;
        }
        
        let upper = self.threshold + self.hysteresis / 2.0;
        let lower = self.threshold - self.hysteresis / 2.0;
        
        let triggered = match self.direction {
            ThresholdDirection::Rising => {
                self.previous < upper && input >= upper
            }
            ThresholdDirection::Falling => {
                self.previous > lower && input <= lower
            }
            ThresholdDirection::Both => {
                (self.previous < upper && input >= upper) ||
                (self.previous > lower && input <= lower)
            }
        };
        
        self.previous = input;
        
        if triggered {
            self.trigger_value
        } else {
            self.rest_value
        }
    }
    
    /// Reset state
    pub fn reset(&mut self) {
        self.initialized = false;
        self.previous = f64::NEG_INFINITY;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_mapper_rising() {
        let mapper = ThresholdMapper::new("test", 50.0)
            .with_direction(ThresholdDirection::Rising);
        
        assert_eq!(mapper.map(40.0), 0.0);
        assert_eq!(mapper.map(50.0), 1.0);
        assert_eq!(mapper.map(60.0), 1.0);
    }

    #[test]
    fn test_threshold_mapper_falling() {
        let mapper = ThresholdMapper::new("test", 50.0)
            .with_direction(ThresholdDirection::Falling);
        
        assert_eq!(mapper.map(60.0), 0.0);
        assert_eq!(mapper.map(50.0), 1.0);
        assert_eq!(mapper.map(40.0), 1.0);
    }

    #[test]
    fn test_threshold_mapper_both() {
        let mapper = ThresholdMapper::new("test", 50.0)
            .with_direction(ThresholdDirection::Both);
        
        assert_eq!(mapper.map(50.0), 1.0); // At threshold counts as "at or beyond"
        assert_eq!(mapper.map(40.0), 1.0); // Below
        assert_eq!(mapper.map(60.0), 1.0); // Above
    }

    #[test]
    fn test_threshold_mapper_custom_values() {
        let mapper = ThresholdMapper::new("test", 50.0)
            .with_trigger_value(440.0)
            .with_rest_value(0.0);
        
        assert_eq!(mapper.map(60.0), 440.0);
        assert_eq!(mapper.map(40.0), 0.0);
    }

    #[test]
    fn test_threshold_mapper_hysteresis() {
        let mapper = ThresholdMapper::new("test", 50.0)
            .with_direction(ThresholdDirection::Rising)
            .with_hysteresis(10.0);
        
        // Threshold is now 55 for rising
        assert_eq!(mapper.map(50.0), 0.0);
        assert_eq!(mapper.map(54.0), 0.0);
        assert_eq!(mapper.map(55.0), 1.0);
        assert_eq!(mapper.map(60.0), 1.0);
    }

    #[test]
    fn test_edge_threshold_mapper() {
        let mut mapper = EdgeThresholdMapper::new("test", 50.0)
            .with_direction(ThresholdDirection::Rising);
        
        // First sample initializes
        assert_eq!(mapper.process(40.0), 0.0);
        
        // Rising edge
        assert_eq!(mapper.process(60.0), 1.0);
        
        // Staying above - no trigger
        assert_eq!(mapper.process(70.0), 0.0);
        
        // Going down then up again - should trigger
        assert_eq!(mapper.process(40.0), 0.0);
        assert_eq!(mapper.process(60.0), 1.0);
    }

    #[test]
    fn test_edge_threshold_mapper_falling() {
        let mut mapper = EdgeThresholdMapper::new("test", 50.0)
            .with_direction(ThresholdDirection::Falling);
        
        assert_eq!(mapper.process(60.0), 0.0); // Initialize
        assert_eq!(mapper.process(40.0), 1.0); // Falling edge
        assert_eq!(mapper.process(30.0), 0.0); // Still below, no trigger
    }
}
