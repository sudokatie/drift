//! Built-in preset configurations for Drift

/// A built-in preset configuration
pub struct Preset {
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub config: &'static str,
}

impl Preset {
    /// Get all built-in presets
    pub fn all() -> Vec<&'static Preset> {
        vec![
            &WEATHER_DRONE,
            &SYSTEM_PULSE,
            &DEEP_SPACE,
            &FOREST_AMBIENT,
            &OCEAN_WAVES,
            &CITY_NOISE,
            &MEDITATION,
            &GLITCH_AMBIENT,
            &NIGHT_WIND,
            &RAIN_MOOD,
        ]
    }

    /// Get a preset by name (case-insensitive)
    pub fn by_name(name: &str) -> Option<&'static Preset> {
        Self::all()
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Get presets by category
    pub fn by_category(category: &str) -> Vec<&'static Preset> {
        Self::all()
            .into_iter()
            .filter(|p| p.category.eq_ignore_ascii_case(category))
            .collect()
    }

    /// Get all unique categories
    pub fn categories() -> Vec<&'static str> {
        let mut cats: Vec<_> = Self::all()
            .iter()
            .map(|p| p.category)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        cats.sort();
        cats
    }
}

// Weather-based ambient drone
pub static WEATHER_DRONE: Preset = Preset {
    name: "weather-drone",
    description: "Atmospheric drone that responds to current weather conditions",
    category: "environmental",
    config: r#"# Weather Drone - Atmospheric sounds driven by weather data
# Temperature affects pitch, humidity affects filter, wind adds noise

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.6

sources:
  - name: weather
    kind: weather
    poll_interval_secs: 300
    weather:
      api_key: ${OPENWEATHER_API_KEY}
      location: "New York,US"

layers:
  - name: main_drone
    source: weather
    voice: drone
    mappings:
      - from: temperature
        to: pitch
        mapper:
          kind: linear
          input_min: -10.0
          input_max: 35.0
          output_min: 80.0
          output_max: 220.0
      - from: humidity
        to: filter_cutoff
        mapper:
          kind: logarithmic
          input_min: 0.0
          input_max: 100.0
          output_min: 200.0
          output_max: 4000.0
      - from: wind_speed
        to: noise_mix
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 20.0
          output_min: 0.0
          output_max: 0.3
"#,
};

// System metrics pulse
pub static SYSTEM_PULSE: Preset = Preset {
    name: "system-pulse",
    description: "CPU and memory drive rhythmic ambient textures",
    category: "data",
    config: r#"# System Pulse - Your computer's heartbeat as music
# CPU affects intensity, memory affects timbre

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.5

sources:
  - name: system
    kind: system
    poll_interval_secs: 1

layers:
  - name: cpu_layer
    source: system
    voice: drone
    mappings:
      - from: cpu_percent
        to: pitch
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 110.0
          output_max: 440.0
      - from: cpu_percent
        to: volume
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 0.2
          output_max: 0.8
      - from: memory_percent
        to: filter_cutoff
        mapper:
          kind: logarithmic
          input_min: 0.0
          input_max: 100.0
          output_min: 300.0
          output_max: 6000.0
"#,
};

// Deep space ambient
pub static DEEP_SPACE: Preset = Preset {
    name: "deep-space",
    description: "Low, slow drones evoking cosmic vastness",
    category: "atmosphere",
    config: r#"# Deep Space - Slow, evolving cosmic drones
# Minimal data input, maximum atmosphere

audio:
  sample_rate: 44100
  buffer_size: 1024

master:
  volume: 0.4

sources:
  - name: system
    kind: system
    poll_interval_secs: 10

layers:
  - name: void_drone
    source: system
    voice: drone
    mappings:
      - from: memory_percent
        to: pitch
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 30.0
          output_max: 60.0
      - from: cpu_percent
        to: filter_cutoff
        mapper:
          kind: logarithmic
          input_min: 0.0
          input_max: 100.0
          output_min: 100.0
          output_max: 800.0
      - from: cpu_percent
        to: noise_mix
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 0.05
          output_max: 0.15
"#,
};

// Forest ambient
pub static FOREST_AMBIENT: Preset = Preset {
    name: "forest-ambient",
    description: "Peaceful woodland soundscape",
    category: "nature",
    config: r#"# Forest Ambient - Gentle, organic textures
# Weather drives subtle variations

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.5

sources:
  - name: weather
    kind: weather
    poll_interval_secs: 600
    weather:
      api_key: ${OPENWEATHER_API_KEY}
      location: "Seattle,US"

layers:
  - name: canopy
    source: weather
    voice: drone
    mappings:
      - from: temperature
        to: pitch
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 30.0
          output_min: 150.0
          output_max: 300.0
      - from: clouds
        to: filter_cutoff
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 800.0
          output_max: 2000.0
      - from: humidity
        to: noise_mix
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 0.02
          output_max: 0.1
"#,
};

// Ocean waves
pub static OCEAN_WAVES: Preset = Preset {
    name: "ocean-waves",
    description: "Rolling, rhythmic oceanic sounds",
    category: "nature",
    config: r#"# Ocean Waves - Deep, rolling ambient
# System load creates wave-like swells

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.55

sources:
  - name: system
    kind: system
    poll_interval_secs: 2

layers:
  - name: waves
    source: system
    voice: drone
    mappings:
      - from: cpu_percent
        to: pitch
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 60.0
          output_max: 120.0
      - from: memory_percent
        to: filter_cutoff
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 200.0
          output_max: 1200.0
      - from: cpu_percent
        to: noise_mix
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 0.1
          output_max: 0.4
"#,
};

// City noise
pub static CITY_NOISE: Preset = Preset {
    name: "city-noise",
    description: "Urban ambient textures with gritty character",
    category: "urban",
    config: r#"# City Noise - Urban ambient with industrial edge
# Higher noise, grittier textures

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.45

sources:
  - name: system
    kind: system
    poll_interval_secs: 1

layers:
  - name: traffic
    source: system
    voice: drone
    mappings:
      - from: cpu_percent
        to: pitch
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 80.0
          output_max: 200.0
      - from: memory_percent
        to: filter_cutoff
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 500.0
          output_max: 3000.0
      - from: cpu_percent
        to: noise_mix
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 0.15
          output_max: 0.5
"#,
};

// Meditation
pub static MEDITATION: Preset = Preset {
    name: "meditation",
    description: "Calm, centered tones for focus and relaxation",
    category: "wellness",
    config: r#"# Meditation - Stable, calming drones
# Minimal variation, maximum serenity

audio:
  sample_rate: 44100
  buffer_size: 1024

master:
  volume: 0.35

sources:
  - name: system
    kind: system
    poll_interval_secs: 30

layers:
  - name: om
    source: system
    voice: drone
    mappings:
      - from: memory_percent
        to: pitch
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 108.0
          output_max: 116.0
      - from: cpu_percent
        to: filter_cutoff
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 400.0
          output_max: 800.0
      - from: memory_percent
        to: noise_mix
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 0.0
          output_max: 0.02
"#,
};

// Glitch ambient
pub static GLITCH_AMBIENT: Preset = Preset {
    name: "glitch-ambient",
    description: "Digital artifacts and stuttering textures",
    category: "experimental",
    config: r#"# Glitch Ambient - Digital artifacts as music
# Fast polling, erratic changes

audio:
  sample_rate: 44100
  buffer_size: 256

master:
  volume: 0.4

sources:
  - name: system
    kind: system
    poll_interval_secs: 1

layers:
  - name: glitch
    source: system
    voice: drone
    mappings:
      - from: cpu_percent
        to: pitch
        mapper:
          kind: quantize
          input_min: 0.0
          input_max: 100.0
          output_min: 100.0
          output_max: 800.0
          scale: whole_tone
      - from: memory_percent
        to: filter_cutoff
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 200.0
          output_max: 8000.0
      - from: cpu_percent
        to: noise_mix
        mapper:
          kind: threshold
          threshold: 50.0
          below_value: 0.1
          above_value: 0.4
"#,
};

// Night wind
pub static NIGHT_WIND: Preset = Preset {
    name: "night-wind",
    description: "Gentle nocturnal breezes and distant drones",
    category: "nature",
    config: r#"# Night Wind - Whispered ambience
# Wind-driven, ethereal textures

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.45

sources:
  - name: weather
    kind: weather
    poll_interval_secs: 300
    weather:
      api_key: ${OPENWEATHER_API_KEY}
      location: "Denver,US"

layers:
  - name: breeze
    source: weather
    voice: drone
    mappings:
      - from: wind_speed
        to: pitch
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 15.0
          output_min: 100.0
          output_max: 180.0
      - from: wind_speed
        to: noise_mix
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 15.0
          output_min: 0.1
          output_max: 0.35
      - from: temperature
        to: filter_cutoff
        mapper:
          kind: linear
          input_min: -5.0
          input_max: 25.0
          output_min: 300.0
          output_max: 1500.0
"#,
};

// Rain mood
pub static RAIN_MOOD: Preset = Preset {
    name: "rain-mood",
    description: "Melancholic rainy day atmosphere",
    category: "nature",
    config: r#"# Rain Mood - Contemplative rainy ambience
# Humidity and clouds drive the mood

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.5

sources:
  - name: weather
    kind: weather
    poll_interval_secs: 300
    weather:
      api_key: ${OPENWEATHER_API_KEY}
      location: "London,GB"

layers:
  - name: rain
    source: weather
    voice: drone
    mappings:
      - from: humidity
        to: pitch
        mapper:
          kind: linear
          input_min: 40.0
          input_max: 100.0
          output_min: 130.0
          output_max: 200.0
      - from: humidity
        to: noise_mix
        mapper:
          kind: linear
          input_min: 40.0
          input_max: 100.0
          output_min: 0.05
          output_max: 0.25
      - from: clouds
        to: filter_cutoff
        mapper:
          kind: linear
          input_min: 0.0
          input_max: 100.0
          output_min: 600.0
          output_max: 1800.0
"#,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets() {
        let presets = Preset::all();
        assert_eq!(presets.len(), 10);
    }

    #[test]
    fn test_by_name() {
        let preset = Preset::by_name("weather-drone").unwrap();
        assert_eq!(preset.name, "weather-drone");
        
        // Case insensitive
        let preset = Preset::by_name("WEATHER-DRONE").unwrap();
        assert_eq!(preset.name, "weather-drone");
    }

    #[test]
    fn test_by_name_not_found() {
        assert!(Preset::by_name("nonexistent").is_none());
    }

    #[test]
    fn test_by_category() {
        let nature = Preset::by_category("nature");
        assert_eq!(nature.len(), 4); // forest, ocean, night-wind, rain
    }

    #[test]
    fn test_categories() {
        let cats = Preset::categories();
        assert!(cats.contains(&"nature"));
        assert!(cats.contains(&"data"));
        assert!(cats.contains(&"atmosphere"));
    }

    #[test]
    fn test_preset_configs_are_valid_yaml() {
        for preset in Preset::all() {
            // Should parse as YAML without error
            let result: Result<serde_yaml::Value, _> = serde_yaml::from_str(preset.config);
            assert!(result.is_ok(), "Preset {} has invalid YAML: {:?}", preset.name, result.err());
        }
    }
}
