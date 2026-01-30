//! Configuration schema definitions

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration for Drift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftConfig {
    /// Audio output settings
    pub audio: AudioConfig,
    
    /// Master settings (tempo, key, volume)
    pub master: MasterConfig,
    
    /// Data sources
    #[serde(default)]
    pub sources: Vec<SourceConfig>,
    
    /// Sound layers
    #[serde(default)]
    pub layers: Vec<LayerConfig>,
}

impl DriftConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate audio settings
        if self.audio.sample_rate < 8000 || self.audio.sample_rate > 192000 {
            bail!("Sample rate must be between 8000 and 192000");
        }
        if self.audio.buffer_size < 64 || self.audio.buffer_size > 8192 {
            bail!("Buffer size must be between 64 and 8192");
        }
        
        // Validate master settings
        if self.master.volume < 0.0 || self.master.volume > 1.0 {
            bail!("Master volume must be between 0.0 and 1.0");
        }
        if self.master.bpm < 20.0 || self.master.bpm > 300.0 {
            bail!("BPM must be between 20 and 300");
        }
        
        // Validate layers reference existing sources
        for layer in &self.layers {
            if !self.sources.iter().any(|s| s.name == layer.source) {
                bail!("Layer '{}' references unknown source '{}'", layer.name, layer.source);
            }
        }
        
        Ok(())
    }
}

/// Audio output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Sample rate in Hz (default: 44100)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    
    /// Buffer size in samples (default: 512)
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    
    /// Output device name (None = default device)
    pub device: Option<String>,
    
    /// Output file path (for recording)
    pub output_file: Option<String>,
}

fn default_sample_rate() -> u32 { 44100 }
fn default_buffer_size() -> usize { 512 }

/// Master settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterConfig {
    /// Beats per minute (default: 60)
    #[serde(default = "default_bpm")]
    pub bpm: f32,
    
    /// Musical key (default: C)
    #[serde(default = "default_key")]
    pub key: String,
    
    /// Scale type (default: minor_pentatonic)
    #[serde(default = "default_scale")]
    pub scale: String,
    
    /// Master volume 0.0-1.0 (default: 0.7)
    #[serde(default = "default_volume")]
    pub volume: f32,
}

fn default_bpm() -> f32 { 60.0 }
fn default_key() -> String { "C".to_string() }
fn default_scale() -> String { "minor_pentatonic".to_string() }
fn default_volume() -> f32 { 0.7 }

/// Data source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Unique name for this source
    pub name: String,
    
    /// Source type
    pub kind: SourceKind,
    
    /// Whether this source is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    
    /// Source-specific settings
    #[serde(default)]
    pub settings: HashMap<String, serde_yaml::Value>,
}

fn default_enabled() -> bool { true }

/// Types of data sources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Weather data from API
    Weather,
    /// Local system metrics
    System,
    /// Git repository events
    Git,
    /// Price data from API
    Price,
}

/// Sound layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerConfig {
    /// Unique name for this layer
    pub name: String,
    
    /// Voice type for this layer
    pub voice: VoiceKind,
    
    /// Name of the data source
    pub source: String,
    
    /// Parameter mappings (parameter_name -> source_field)
    #[serde(default)]
    pub mappings: HashMap<String, MappingConfig>,
    
    /// Layer volume 0.0-1.0 (default: 1.0)
    #[serde(default = "default_layer_volume")]
    pub volume: f32,
}

fn default_layer_volume() -> f32 { 1.0 }

/// Types of voices (sound generators)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VoiceKind {
    /// Sustained tones
    Drone,
    /// Short triggers
    Percussion,
    /// Melodic patterns
    Melody,
    /// Noise and grain
    Texture,
}

/// Mapping configuration for a parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingConfig {
    /// Source field to map from
    pub field: String,
    
    /// Mapping type
    #[serde(default)]
    pub kind: MappingKind,
    
    /// Input range minimum
    pub in_min: Option<f64>,
    
    /// Input range maximum
    pub in_max: Option<f64>,
    
    /// Output range minimum
    pub out_min: Option<f64>,
    
    /// Output range maximum
    pub out_max: Option<f64>,
}

/// Types of mapping functions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MappingKind {
    /// Linear interpolation (default)
    #[default]
    Linear,
    /// Logarithmic scaling
    Logarithmic,
    /// Exponential scaling
    Exponential,
    /// Threshold trigger
    Threshold,
    /// Quantize to scale degrees
    Quantize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_audio_config() {
        let yaml = "sample_rate: 48000";
        let config: AudioConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.buffer_size, 512); // default
    }

    #[test]
    fn test_source_config() {
        let yaml = r#"
name: weather
kind: weather
enabled: true
settings:
  api_key: test123
  location: "Austin, TX"
"#;
        let config: SourceConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "weather");
        assert_eq!(config.kind, SourceKind::Weather);
        assert!(config.enabled);
    }

    #[test]
    fn test_layer_config() {
        let yaml = r#"
name: weather_drone
voice: drone
source: weather
volume: 0.8
mappings:
  pitch:
    field: temperature
    kind: linear
    in_min: -20
    in_max: 40
    out_min: 100
    out_max: 400
"#;
        let config: LayerConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.name, "weather_drone");
        assert_eq!(config.voice, VoiceKind::Drone);
        assert_eq!(config.volume, 0.8);
        assert!(config.mappings.contains_key("pitch"));
    }

    #[test]
    fn test_config_validation() {
        let config = DriftConfig {
            audio: AudioConfig {
                sample_rate: 44100,
                buffer_size: 512,
                device: None,
                output_file: None,
            },
            master: MasterConfig {
                bpm: 60.0,
                key: "C".to_string(),
                scale: "minor_pentatonic".to_string(),
                volume: 0.7,
            },
            sources: vec![
                SourceConfig {
                    name: "weather".to_string(),
                    kind: SourceKind::Weather,
                    enabled: true,
                    settings: HashMap::new(),
                }
            ],
            layers: vec![
                LayerConfig {
                    name: "drone".to_string(),
                    voice: VoiceKind::Drone,
                    source: "weather".to_string(),
                    mappings: HashMap::new(),
                    volume: 1.0,
                }
            ],
        };
        
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_layer_source() {
        let config = DriftConfig {
            audio: AudioConfig {
                sample_rate: 44100,
                buffer_size: 512,
                device: None,
                output_file: None,
            },
            master: MasterConfig {
                bpm: 60.0,
                key: "C".to_string(),
                scale: "minor_pentatonic".to_string(),
                volume: 0.7,
            },
            sources: vec![],
            layers: vec![
                LayerConfig {
                    name: "drone".to_string(),
                    voice: VoiceKind::Drone,
                    source: "nonexistent".to_string(),
                    mappings: HashMap::new(),
                    volume: 1.0,
                }
            ],
        };
        
        assert!(config.validate().is_err());
    }
}
