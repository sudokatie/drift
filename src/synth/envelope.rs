//! ADSR envelope generator
//!
//! Attack-Decay-Sustain-Release envelope for amplitude shaping.

/// Envelope stage
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// ADSR envelope generator
pub struct Envelope {
    sample_rate: f64,
    
    // Time parameters (in seconds)
    attack: f64,
    decay: f64,
    sustain: f64,  // Level (0.0-1.0)
    release: f64,
    
    // State
    stage: EnvelopeStage,
    level: f64,
    time_in_stage: f64,
    release_start_level: f64,
}

impl Envelope {
    /// Create a new envelope with default parameters
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            attack: 0.01,    // 10ms
            decay: 0.1,      // 100ms
            sustain: 0.7,    // 70% level
            release: 0.3,    // 300ms
            stage: EnvelopeStage::Idle,
            level: 0.0,
            time_in_stage: 0.0,
            release_start_level: 0.0,
        }
    }
    
    /// Set attack time in seconds
    pub fn set_attack(&mut self, seconds: f64) {
        self.attack = seconds.max(0.001); // Minimum 1ms
    }
    
    /// Set decay time in seconds
    pub fn set_decay(&mut self, seconds: f64) {
        self.decay = seconds.max(0.001);
    }
    
    /// Set sustain level (0.0-1.0)
    pub fn set_sustain(&mut self, level: f64) {
        self.sustain = level.clamp(0.0, 1.0);
    }
    
    /// Set release time in seconds
    pub fn set_release(&mut self, seconds: f64) {
        self.release = seconds.max(0.001);
    }
    
    /// Configure all ADSR parameters at once
    pub fn configure(&mut self, attack: f64, decay: f64, sustain: f64, release: f64) {
        self.set_attack(attack);
        self.set_decay(decay);
        self.set_sustain(sustain);
        self.set_release(release);
    }
    
    /// Trigger the envelope (start attack phase)
    pub fn trigger(&mut self) {
        self.stage = EnvelopeStage::Attack;
        self.time_in_stage = 0.0;
        // Don't reset level - allows retriggering from current position
    }
    
    /// Release the envelope (start release phase)
    pub fn release(&mut self) {
        if self.stage != EnvelopeStage::Idle && self.stage != EnvelopeStage::Release {
            self.release_start_level = self.level;
            self.stage = EnvelopeStage::Release;
            self.time_in_stage = 0.0;
        }
    }
    
    /// Reset envelope to idle state
    pub fn reset(&mut self) {
        self.stage = EnvelopeStage::Idle;
        self.level = 0.0;
        self.time_in_stage = 0.0;
    }
    
    /// Get current stage
    pub fn stage(&self) -> EnvelopeStage {
        self.stage
    }
    
    /// Check if envelope is active (not idle)
    pub fn is_active(&self) -> bool {
        self.stage != EnvelopeStage::Idle
    }
    
    /// Generate the next envelope sample
    pub fn process(&mut self) -> f64 {
        let dt = 1.0 / self.sample_rate;
        
        match self.stage {
            EnvelopeStage::Idle => {
                self.level = 0.0;
            }
            
            EnvelopeStage::Attack => {
                // Linear attack from current level to 1.0
                self.level += dt / self.attack;
                self.time_in_stage += dt;
                
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                    self.time_in_stage = 0.0;
                }
            }
            
            EnvelopeStage::Decay => {
                // Exponential decay from 1.0 to sustain level
                let target = self.sustain;
                let rate = (1.0 - target) / self.decay;
                self.level -= rate * dt;
                self.time_in_stage += dt;
                
                if self.level <= target {
                    self.level = target;
                    self.stage = EnvelopeStage::Sustain;
                    self.time_in_stage = 0.0;
                }
            }
            
            EnvelopeStage::Sustain => {
                // Hold at sustain level
                self.level = self.sustain;
            }
            
            EnvelopeStage::Release => {
                // Linear release from release_start_level to 0.0
                let rate = self.release_start_level / self.release;
                self.level -= rate * dt;
                self.time_in_stage += dt;
                
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                    self.time_in_stage = 0.0;
                }
            }
        }
        
        self.level
    }
    
    /// Get current level without advancing
    pub fn level(&self) -> f64 {
        self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_creation() {
        let env = Envelope::new(44100.0);
        assert_eq!(env.stage(), EnvelopeStage::Idle);
        assert_eq!(env.level(), 0.0);
        assert!(!env.is_active());
    }

    #[test]
    fn test_envelope_trigger() {
        let mut env = Envelope::new(44100.0);
        env.trigger();
        
        assert_eq!(env.stage(), EnvelopeStage::Attack);
        assert!(env.is_active());
    }

    #[test]
    fn test_envelope_attack_phase() {
        let mut env = Envelope::new(44100.0);
        env.set_attack(0.01); // 10ms attack
        env.trigger();
        
        // Process through attack (441 samples for 10ms at 44100 Hz)
        for _ in 0..500 {
            env.process();
        }
        
        // Should be at or near 1.0 (in decay or sustain)
        assert!(env.level() > 0.9);
    }

    #[test]
    fn test_envelope_sustain_level() {
        let mut env = Envelope::new(44100.0);
        env.configure(0.001, 0.001, 0.5, 0.001); // Very fast ADSR
        env.trigger();
        
        // Process through attack and decay
        for _ in 0..500 {
            env.process();
        }
        
        // Should be at sustain level
        assert!((env.level() - 0.5).abs() < 0.01);
        assert_eq!(env.stage(), EnvelopeStage::Sustain);
    }

    #[test]
    fn test_envelope_release() {
        let mut env = Envelope::new(44100.0);
        env.configure(0.001, 0.001, 0.5, 0.01); // 10ms release
        env.trigger();
        
        // Process to sustain
        for _ in 0..200 {
            env.process();
        }
        
        // Trigger release
        env.release();
        assert_eq!(env.stage(), EnvelopeStage::Release);
        
        // Process through release
        for _ in 0..1000 {
            env.process();
        }
        
        // Should be at 0 and idle
        assert_eq!(env.level(), 0.0);
        assert_eq!(env.stage(), EnvelopeStage::Idle);
    }

    #[test]
    fn test_envelope_reset() {
        let mut env = Envelope::new(44100.0);
        env.trigger();
        
        for _ in 0..100 {
            env.process();
        }
        
        env.reset();
        assert_eq!(env.stage(), EnvelopeStage::Idle);
        assert_eq!(env.level(), 0.0);
    }

    #[test]
    fn test_envelope_configure() {
        let mut env = Envelope::new(44100.0);
        env.configure(0.5, 0.3, 0.6, 0.8);
        
        // Start envelope and verify timing behavior
        env.trigger();
        assert_eq!(env.stage(), EnvelopeStage::Attack);
    }
}
