//! Configuration loading and validation

mod schema;

pub use schema::*;

use anyhow::Result;
use std::path::Path;

/// Load configuration from a YAML file
pub fn load_config(path: &Path) -> Result<DriftConfig> {
    let contents = std::fs::read_to_string(path)?;
    let config: DriftConfig = serde_yaml::from_str(&contents)?;
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_load_minimal_config() {
        let yaml = r#"
audio:
  sample_rate: 44100
  buffer_size: 512

master:
  volume: 0.7

sources: []
layers: []
"#;
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        
        let config = load_config(file.path()).unwrap();
        assert_eq!(config.audio.sample_rate, 44100);
        assert_eq!(config.master.volume, 0.7);
    }
}
