//! Voice trait for sound generators

/// Trait for voice implementations
pub trait Voice: Send + Sync {
    /// Set a parameter value
    fn set_parameter(&mut self, name: &str, value: f64);
    
    /// Get a parameter value
    fn get_parameter(&self, name: &str) -> Option<f64>;
    
    /// Trigger the voice (start a note)
    fn trigger(&mut self);
    
    /// Release the voice (end a note)
    fn release(&mut self);
    
    /// Check if the voice is currently active
    fn is_active(&self) -> bool;
    
    /// Generate the next sample
    fn process(&mut self) -> f64;
    
    /// Set the sample rate
    fn set_sample_rate(&mut self, sample_rate: f64);
}
