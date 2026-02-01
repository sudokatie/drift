//! Pattern mapper implementation
//!
//! Converts time series data into rhythmic patterns using Euclidean rhythms.
//! Useful for generating percussion triggers from continuous data.

use super::Mapper;

/// Euclidean rhythm pattern generator
///
/// Euclidean rhythms distribute `pulses` hits as evenly as possible
/// over `steps` total steps. This creates musically interesting patterns
/// that appear in many world music traditions.
#[derive(Debug, Clone)]
pub struct EuclideanPattern {
    /// Total steps in the pattern
    steps: usize,
    /// Number of pulses (hits)
    pulses: usize,
    /// Current position in the pattern
    position: usize,
    /// Pre-computed pattern
    pattern: Vec<bool>,
}

impl EuclideanPattern {
    /// Create a new Euclidean rhythm pattern
    ///
    /// # Arguments
    /// * `pulses` - Number of hits in the pattern
    /// * `steps` - Total number of steps
    ///
    /// # Example
    /// ```
    /// use drift::mapping::EuclideanPattern;
    /// let pattern = EuclideanPattern::new(3, 8); // Creates [X . . X . . X .]
    /// assert_eq!(pattern.pulses(), 3);
    /// assert_eq!(pattern.steps(), 8);
    /// ```
    pub fn new(pulses: usize, steps: usize) -> Self {
        let pattern = Self::generate_pattern(pulses, steps);
        Self {
            steps,
            pulses,
            position: 0,
            pattern,
        }
    }

    /// Generate the Euclidean pattern using Bjorklund's algorithm
    fn generate_pattern(pulses: usize, steps: usize) -> Vec<bool> {
        if steps == 0 {
            return vec![];
        }
        if pulses == 0 {
            return vec![false; steps];
        }
        if pulses >= steps {
            return vec![true; steps];
        }

        // Bjorklund's algorithm
        let mut pattern = Vec::with_capacity(steps);

        // Start with pulses number of [true] and (steps-pulses) number of [false]
        let mut groups: Vec<Vec<bool>> = Vec::new();
        for _ in 0..pulses {
            groups.push(vec![true]);
        }
        for _ in 0..(steps - pulses) {
            groups.push(vec![false]);
        }

        // Iteratively merge groups
        while groups.len() > 1 {
            let ones: Vec<Vec<bool>> = groups
                .iter()
                .filter(|g| g.contains(&true))
                .cloned()
                .collect();
            let zeros: Vec<Vec<bool>> = groups
                .iter()
                .filter(|g| !g.contains(&true))
                .cloned()
                .collect();

            if zeros.is_empty() || ones.is_empty() {
                break;
            }

            let min_len = ones.len().min(zeros.len());
            let mut new_groups: Vec<Vec<bool>> = Vec::new();

            // Merge pairs
            for i in 0..min_len {
                let mut merged = ones[i].clone();
                merged.extend(zeros[i].clone());
                new_groups.push(merged);
            }

            // Add remainder
            if ones.len() > zeros.len() {
                for g in ones.iter().skip(min_len) {
                    new_groups.push(g.clone());
                }
            } else {
                for g in zeros.iter().skip(min_len) {
                    new_groups.push(g.clone());
                }
            }

            groups = new_groups;

            // Break if we can't reduce further
            if groups.iter().all(|g| g.len() == groups[0].len()) && groups.len() <= pulses {
                break;
            }
        }

        // Flatten groups into pattern
        for group in groups {
            pattern.extend(group);
        }

        // Ensure pattern is exactly `steps` long
        pattern.truncate(steps);
        while pattern.len() < steps {
            pattern.push(false);
        }

        pattern
    }

    /// Get the current step value (true = hit, false = rest)
    pub fn current(&self) -> bool {
        self.pattern.get(self.position).copied().unwrap_or(false)
    }

    /// Advance to the next step and return whether it's a hit
    pub fn advance(&mut self) -> bool {
        let value = self.current();
        self.position = (self.position + 1) % self.steps.max(1);
        value
    }

    /// Reset to the beginning of the pattern
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Get the full pattern as a slice
    pub fn pattern(&self) -> &[bool] {
        &self.pattern
    }

    /// Get the number of steps
    pub fn steps(&self) -> usize {
        self.steps
    }

    /// Get the number of pulses
    pub fn pulses(&self) -> usize {
        self.pulses
    }
}

/// Pattern mapper that converts continuous data to rhythmic triggers
///
/// Maps input values to Euclidean rhythm densities:
/// - Low values = sparse patterns (few hits)
/// - High values = dense patterns (many hits)
pub struct PatternMapper {
    name: String,
    /// Input range minimum
    in_min: f64,
    /// Input range maximum
    in_max: f64,
    /// Current pattern
    pattern: EuclideanPattern,
    /// Trigger value to output on hits
    trigger_value: f64,
    /// Rest value to output on non-hits
    rest_value: f64,
}

impl PatternMapper {
    /// Create a new pattern mapper
    ///
    /// # Arguments
    /// * `name` - Name for this mapper
    /// * `in_min` - Minimum expected input value
    /// * `in_max` - Maximum expected input value
    /// * `steps` - Number of steps in the generated pattern
    pub fn new(name: impl Into<String>, in_min: f64, in_max: f64, steps: usize) -> Self {
        Self {
            name: name.into(),
            in_min,
            in_max,
            pattern: EuclideanPattern::new(steps / 2, steps),
            trigger_value: 1.0,
            rest_value: 0.0,
        }
    }

    /// Set the trigger value (output on hits)
    pub fn with_trigger_value(mut self, value: f64) -> Self {
        self.trigger_value = value;
        self
    }

    /// Set the rest value (output on non-hits)
    pub fn with_rest_value(mut self, value: f64) -> Self {
        self.rest_value = value;
        self
    }

    /// Update the pattern density based on input value
    pub fn update_pattern(&mut self, input: f64) {
        // Normalize input to 0-1
        let range = self.in_max - self.in_min;
        let normalized = if range.abs() < f64::EPSILON {
            0.5
        } else {
            ((input - self.in_min) / range).clamp(0.0, 1.0)
        };

        // Map to number of pulses (0 to steps)
        let steps = self.pattern.steps();
        let pulses = (normalized * steps as f64).round() as usize;

        // Only recreate if density changed
        if pulses != self.pattern.pulses() {
            self.pattern = EuclideanPattern::new(pulses, steps);
        }
    }

    /// Advance the pattern and return trigger or rest value
    pub fn step(&mut self) -> f64 {
        if self.pattern.advance() {
            self.trigger_value
        } else {
            self.rest_value
        }
    }

    /// Reset the pattern to the beginning
    pub fn reset(&mut self) {
        self.pattern.reset();
    }

    /// Get the current pattern
    pub fn current_pattern(&self) -> &[bool] {
        self.pattern.pattern()
    }

    /// Get number of steps
    pub fn steps(&self) -> usize {
        self.pattern.steps()
    }
}

impl Mapper for PatternMapper {
    fn name(&self) -> &str {
        &self.name
    }

    /// Map input to pattern density and return current step value
    ///
    /// Note: For stateless Mapper trait, this updates the pattern density
    /// based on input but returns the trigger value for consistency.
    /// Use `step()` method for actual pattern stepping.
    fn map(&self, input: f64) -> f64 {
        // For stateless interface, return based on whether this would be dense or sparse
        let range = self.in_max - self.in_min;
        let normalized = if range.abs() < f64::EPSILON {
            0.5
        } else {
            ((input - self.in_min) / range).clamp(0.0, 1.0)
        };

        // High input = more likely to trigger
        if normalized > 0.5 {
            self.trigger_value
        } else {
            self.rest_value
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_euclidean_3_8() {
        // E(3,8) = [X . . X . . X .]
        let pattern = EuclideanPattern::new(3, 8);
        let expected = vec![true, false, false, true, false, false, true, false];

        assert_eq!(pattern.pattern(), &expected);
        assert_eq!(pattern.steps(), 8);
        assert_eq!(pattern.pulses(), 3);
    }

    #[test]
    fn test_euclidean_4_8() {
        // E(4,8) = [X . X . X . X .]
        let pattern = EuclideanPattern::new(4, 8);

        // Count pulses
        let pulse_count = pattern.pattern().iter().filter(|&&x| x).count();
        assert_eq!(pulse_count, 4);
        assert_eq!(pattern.pattern().len(), 8);
    }

    #[test]
    fn test_euclidean_5_8() {
        // E(5,8) = Cuban cinquillo
        let pattern = EuclideanPattern::new(5, 8);

        let pulse_count = pattern.pattern().iter().filter(|&&x| x).count();
        assert_eq!(pulse_count, 5);
    }

    #[test]
    fn test_euclidean_empty() {
        let pattern = EuclideanPattern::new(0, 8);
        assert!(pattern.pattern().iter().all(|&x| !x));
    }

    #[test]
    fn test_euclidean_full() {
        let pattern = EuclideanPattern::new(8, 8);
        assert!(pattern.pattern().iter().all(|&x| x));
    }

    #[test]
    fn test_euclidean_zero_steps() {
        let pattern = EuclideanPattern::new(3, 0);
        assert!(pattern.pattern().is_empty());
    }

    #[test]
    fn test_euclidean_next() {
        let mut pattern = EuclideanPattern::new(2, 4);

        // Cycle through pattern twice
        let mut hits = 0;
        for _ in 0..8 {
            if pattern.advance() {
                hits += 1;
            }
        }
        assert_eq!(hits, 4); // 2 hits per cycle * 2 cycles
    }

    #[test]
    fn test_euclidean_reset() {
        let mut pattern = EuclideanPattern::new(2, 4);

        pattern.advance();
        pattern.advance();
        pattern.reset();

        assert_eq!(pattern.position, 0);
    }

    #[test]
    fn test_pattern_mapper_creation() {
        let mapper = PatternMapper::new("test", 0.0, 100.0, 8);

        assert_eq!(mapper.name(), "test");
        assert_eq!(mapper.steps(), 8);
    }

    #[test]
    fn test_pattern_mapper_density() {
        let mut mapper = PatternMapper::new("test", 0.0, 100.0, 8);

        // Low input = sparse pattern
        mapper.update_pattern(10.0);
        let sparse_pulses = mapper.pattern.pulses();

        // High input = dense pattern
        mapper.update_pattern(90.0);
        let dense_pulses = mapper.pattern.pulses();

        assert!(dense_pulses > sparse_pulses);
    }

    #[test]
    fn test_pattern_mapper_step() {
        let mut mapper = PatternMapper::new("test", 0.0, 100.0, 4)
            .with_trigger_value(440.0)
            .with_rest_value(0.0);

        mapper.update_pattern(50.0); // Half density

        // Step through and count hits
        let mut hits = 0;
        for _ in 0..4 {
            if mapper.step() > 0.0 {
                hits += 1;
            }
        }

        assert!(hits > 0 && hits < 4);
    }

    #[test]
    fn test_pattern_mapper_map() {
        let mapper = PatternMapper::new("test", 0.0, 100.0, 8)
            .with_trigger_value(1.0)
            .with_rest_value(0.0);

        // High input should return trigger value
        assert_eq!(mapper.map(80.0), 1.0);

        // Low input should return rest value
        assert_eq!(mapper.map(20.0), 0.0);
    }

    #[test]
    fn test_pattern_mapper_custom_values() {
        let mapper = PatternMapper::new("test", 0.0, 100.0, 8)
            .with_trigger_value(440.0)
            .with_rest_value(-1.0);

        assert_eq!(mapper.map(80.0), 440.0);
        assert_eq!(mapper.map(20.0), -1.0);
    }

    #[test]
    fn test_common_euclidean_rhythms() {
        // These are well-known Euclidean rhythms from world music

        // E(2,5) = Khafif-e-ramal
        let e25 = EuclideanPattern::new(2, 5);
        assert_eq!(e25.pattern().iter().filter(|&&x| x).count(), 2);

        // E(3,7) = Sicilian
        let e37 = EuclideanPattern::new(3, 7);
        assert_eq!(e37.pattern().iter().filter(|&&x| x).count(), 3);

        // E(4,9) = Turkish aksak
        let e49 = EuclideanPattern::new(4, 9);
        assert_eq!(e49.pattern().iter().filter(|&&x| x).count(), 4);

        // E(5,12) = Venda
        let e512 = EuclideanPattern::new(5, 12);
        assert_eq!(e512.pattern().iter().filter(|&&x| x).count(), 5);
    }
}
