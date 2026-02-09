//! Mixer for combining sources, mappings, and voices
//!
//! The Mixer is the core of Drift's audio generation. It:
//! - Receives data from sources
//! - Applies mappings to convert data to audio parameters
//! - Routes parameters to voices
//! - Mixes voice outputs into the final audio stream

use crate::config::{LayerConfig, MappingConfig, MappingKind, VoiceKind};
use crate::mapping::{ExponentialMapper, LinearMapper, LogarithmicMapper, MappingPipeline, QuantizeMapper, Scale, ThresholdMapper, ThresholdDirection};
use crate::sources::DataPoint;
use crate::synth::{DroneVoice, Voice};
use std::collections::HashMap;

/// A layer in the mixer (source -> mappings -> voice)
pub struct MixerLayer {
    /// Layer name
    pub name: String,
    /// Source name this layer listens to
    pub source: String,
    /// Voice for this layer
    voice: Box<dyn Voice>,
    /// Parameter mappings (param_name -> (field_name, pipeline))
    mappings: HashMap<String, (String, MappingPipeline)>,
    /// Layer volume
    volume: f32,
}

impl MixerLayer {
    /// Create a new layer from config
    pub fn new(config: &LayerConfig, sample_rate: f64) -> Self {
        // Create appropriate voice based on config
        let voice: Box<dyn Voice> = match config.voice {
            VoiceKind::Drone => Box::new(DroneVoice::new(sample_rate)),
            // Not yet implemented - fall back to drone with warning
            VoiceKind::Percussion | VoiceKind::Melody | VoiceKind::Texture => {
                eprintln!(
                    "Warning: {:?} voice not yet implemented, using drone",
                    config.voice
                );
                Box::new(DroneVoice::new(sample_rate))
            }
        };
        
        // Build mappings
        let mut mappings = HashMap::new();
        for (param_name, mapping_config) in &config.mappings {
            let pipeline = Self::build_pipeline(mapping_config);
            mappings.insert(
                param_name.clone(),
                (mapping_config.field.clone(), pipeline),
            );
        }
        
        Self {
            name: config.name.clone(),
            source: config.source.clone(),
            voice,
            mappings,
            volume: config.volume,
        }
    }
    
    /// Build a mapping pipeline from config
    fn build_pipeline(config: &MappingConfig) -> MappingPipeline {
        let in_min = config.in_min.unwrap_or(0.0);
        let in_max = config.in_max.unwrap_or(100.0);
        let out_min = config.out_min.unwrap_or(0.0);
        let out_max = config.out_max.unwrap_or(1.0);
        
        match config.kind {
            MappingKind::Linear => {
                MappingPipeline::new()
                    .with(LinearMapper::new("linear", in_min, in_max, out_min, out_max))
            }
            MappingKind::Logarithmic => {
                MappingPipeline::new()
                    .with(LogarithmicMapper::new("logarithmic", in_min, in_max, out_min, out_max))
            }
            MappingKind::Exponential => {
                // True exponential mapper (inverse of logarithmic)
                // Creates a curve where small input changes at low values
                // produce large output changes (slow start, fast finish)
                MappingPipeline::new()
                    .with(ExponentialMapper::new("exponential", in_min, in_max, out_min, out_max))
            }
            MappingKind::Threshold => {
                // Use midpoint of input range as threshold
                let threshold = (in_min + in_max) / 2.0;
                MappingPipeline::new()
                    .with(ThresholdMapper::new("threshold", threshold)
                        .with_direction(ThresholdDirection::Rising)
                        .with_trigger_value(out_max)
                        .with_rest_value(out_min))
            }
            MappingKind::Quantize => {
                // Default to pentatonic scale if not specified
                let scale = Scale::from_name("pentatonic").unwrap_or_else(Scale::minor_pentatonic);
                // Use 220 Hz (A3) as root, map input range to frequency range then quantize
                MappingPipeline::new()
                    .with(LinearMapper::new("range", in_min, in_max, out_min, out_max))
                    .with(QuantizeMapper::new("quantize", 220.0, scale))
            }
        }
    }
    
    /// Process a data point and update voice parameters
    pub fn process_data(&mut self, data: &DataPoint) {
        for (param_name, (field_name, pipeline)) in &self.mappings {
            if let Some(&value) = data.values.get(field_name) {
                let mapped = pipeline.apply(value);
                self.voice.set_parameter(param_name, mapped);
            }
        }
    }
    
    /// Generate the next sample from this layer
    pub fn process(&mut self) -> f64 {
        if self.voice.is_active() {
            self.voice.process() * self.volume as f64
        } else {
            0.0
        }
    }
    
    /// Trigger the voice
    pub fn trigger(&mut self) {
        self.voice.trigger();
    }
    
    /// Release the voice
    pub fn release(&mut self) {
        self.voice.release();
    }
    
    /// Check if the voice is active
    pub fn is_active(&self) -> bool {
        self.voice.is_active()
    }
}

/// The main mixer
pub struct Mixer {
    /// Layers indexed by name
    layers: Vec<MixerLayer>,
    /// Sample rate
    sample_rate: f64,
    /// Master volume
    master_volume: f32,
    /// Latest data from each source
    latest_data: HashMap<String, DataPoint>,
}

impl Mixer {
    /// Create a new mixer
    pub fn new(sample_rate: f64, master_volume: f32) -> Self {
        Self {
            layers: Vec::new(),
            sample_rate,
            master_volume,
            latest_data: HashMap::new(),
        }
    }
    
    /// Get the sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
    
    /// Add a layer from config
    pub fn add_layer(&mut self, config: &LayerConfig) {
        let layer = MixerLayer::new(config, self.sample_rate);
        self.layers.push(layer);
    }
    
    /// Get the number of layers
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }
    
    /// Process incoming data from a source
    pub fn receive_data(&mut self, data: DataPoint) {
        let source_name = data.source.clone();
        
        // Update layers that listen to this source
        for layer in &mut self.layers {
            if layer.source == source_name {
                layer.process_data(&data);
            }
        }
        
        // Store latest data
        self.latest_data.insert(source_name, data);
    }
    
    /// Trigger all layers
    pub fn trigger_all(&mut self) {
        for layer in &mut self.layers {
            layer.trigger();
        }
    }
    
    /// Release all layers
    pub fn release_all(&mut self) {
        for layer in &mut self.layers {
            layer.release();
        }
    }
    
    /// Generate the next mixed sample
    pub fn process(&mut self) -> f64 {
        let mut output = 0.0;
        
        for layer in &mut self.layers {
            output += layer.process();
        }
        
        output * self.master_volume as f64
    }
    
    /// Fill a buffer with mixed audio
    pub fn fill_buffer(&mut self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = self.process() as f32;
        }
    }
    
    /// Get the latest data value for a source and field
    pub fn get_latest(&self, source: &str, field: &str) -> Option<f64> {
        self.latest_data
            .get(source)
            .and_then(|data| data.values.get(field).copied())
    }
    
    /// Check if any layer is active
    pub fn has_active_layers(&self) -> bool {
        self.layers.iter().any(|l| l.is_active())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MappingConfig, MappingKind, VoiceKind};
    use std::collections::HashMap;

    fn test_layer_config() -> LayerConfig {
        let mut mappings = HashMap::new();
        mappings.insert(
            "pitch".to_string(),
            MappingConfig {
                field: "temperature".to_string(),
                kind: MappingKind::Linear,
                in_min: Some(-20.0),
                in_max: Some(40.0),
                out_min: Some(100.0),
                out_max: Some(400.0),
            },
        );
        mappings.insert(
            "filter".to_string(),
            MappingConfig {
                field: "humidity".to_string(),
                kind: MappingKind::Linear,
                in_min: Some(0.0),
                in_max: Some(100.0),
                out_min: Some(200.0),
                out_max: Some(2000.0),
            },
        );
        
        LayerConfig {
            name: "test_drone".to_string(),
            voice: VoiceKind::Drone,
            source: "weather".to_string(),
            mappings,
            volume: 0.8,
        }
    }

    #[test]
    fn test_mixer_creation() {
        let mixer = Mixer::new(44100.0, 0.7);
        assert_eq!(mixer.sample_rate(), 44100.0);
        assert_eq!(mixer.layer_count(), 0);
    }

    #[test]
    fn test_mixer_add_layer() {
        let mut mixer = Mixer::new(44100.0, 0.7);
        mixer.add_layer(&test_layer_config());
        
        assert_eq!(mixer.layer_count(), 1);
    }

    #[test]
    fn test_mixer_receive_data() {
        let mut mixer = Mixer::new(44100.0, 0.7);
        mixer.add_layer(&test_layer_config());
        mixer.trigger_all();
        
        let data = DataPoint::new("weather")
            .with_value("temperature", 22.5)
            .with_value("humidity", 65.0);
        
        mixer.receive_data(data);
        
        // Check latest data is stored
        assert_eq!(mixer.get_latest("weather", "temperature"), Some(22.5));
        assert_eq!(mixer.get_latest("weather", "humidity"), Some(65.0));
    }

    #[test]
    fn test_mixer_process() {
        let mut mixer = Mixer::new(44100.0, 0.7);
        mixer.add_layer(&test_layer_config());
        mixer.trigger_all();
        
        // Process some samples and verify we get audio
        let mut max_sample = 0.0f64;
        for _ in 0..1000 {
            let sample = mixer.process();
            max_sample = max_sample.max(sample.abs());
        }
        
        assert!(max_sample > 0.0, "Expected non-zero audio output");
    }

    #[test]
    fn test_mixer_fill_buffer() {
        let mut mixer = Mixer::new(44100.0, 0.7);
        mixer.add_layer(&test_layer_config());
        mixer.trigger_all();
        
        let mut buffer = vec![0.0f32; 512];
        mixer.fill_buffer(&mut buffer);
        
        let has_audio = buffer.iter().any(|&s| s.abs() > 0.0);
        assert!(has_audio, "Buffer should contain audio");
    }

    #[test]
    fn test_mixer_data_to_voice_parameters() {
        let mut mixer = Mixer::new(44100.0, 0.7);
        mixer.add_layer(&test_layer_config());
        mixer.trigger_all();
        
        // Send weather data
        let data = DataPoint::new("weather")
            .with_value("temperature", 22.5); // Should map to ~254 Hz
        
        mixer.receive_data(data);
        
        // Generate samples - the pitch should have changed
        // We can't easily verify pitch, but we can verify it runs
        for _ in 0..100 {
            mixer.process();
        }
    }

    #[test]
    fn test_mixer_multiple_layers() {
        let mut mixer = Mixer::new(44100.0, 0.7);
        
        // Add two layers
        let config1 = test_layer_config();
        let mut config2 = test_layer_config();
        config2.name = "test_drone_2".to_string();
        config2.source = "system".to_string();
        
        mixer.add_layer(&config1);
        mixer.add_layer(&config2);
        
        assert_eq!(mixer.layer_count(), 2);
        
        mixer.trigger_all();
        
        // Send data to each source
        mixer.receive_data(DataPoint::new("weather").with_value("temperature", 20.0));
        mixer.receive_data(DataPoint::new("system").with_value("cpu_percent", 50.0));
        
        // Should have both data points
        assert!(mixer.get_latest("weather", "temperature").is_some());
    }

    #[test]
    fn test_mixer_layer_volume() {
        let mut mixer = Mixer::new(44100.0, 1.0); // Master volume 1.0
        
        // Create layer with 0 volume
        let mut config = test_layer_config();
        config.volume = 0.0;
        mixer.add_layer(&config);
        mixer.trigger_all();
        
        // Output should be silent
        for _ in 0..100 {
            let sample = mixer.process();
            assert_eq!(sample, 0.0);
        }
    }

    #[test]
    fn test_mixer_trigger_release() {
        let mut mixer = Mixer::new(44100.0, 0.7);
        mixer.add_layer(&test_layer_config());
        
        // DroneVoice starts active by default (for sustained drones)
        assert!(mixer.has_active_layers());
        
        mixer.release_all();
        
        // With ADSR envelope, voice stays active during release phase
        // Process samples to let release complete (1s release at 44100 Hz)
        for _ in 0..50000 {
            mixer.process();
        }
        
        assert!(!mixer.has_active_layers());
        
        mixer.trigger_all();
        assert!(mixer.has_active_layers());
    }

    #[test]
    fn test_layer_creation() {
        let config = test_layer_config();
        let layer = MixerLayer::new(&config, 44100.0);
        
        assert_eq!(layer.name, "test_drone");
        assert_eq!(layer.source, "weather");
        // DroneVoice starts active by default (for sustained drones)
        assert!(layer.is_active());
    }

    #[test]
    fn test_layer_process_data() {
        let config = test_layer_config();
        let mut layer = MixerLayer::new(&config, 44100.0);
        layer.trigger();
        
        let data = DataPoint::new("weather")
            .with_value("temperature", 10.0)
            .with_value("humidity", 50.0);
        
        layer.process_data(&data);
        
        // Voice parameters should be updated (we can't easily verify the values)
        // but the layer should still be active
        assert!(layer.is_active());
    }
}
