//! Configuration loading and validation

mod schema;

pub use schema::*;

use anyhow::Result;
use std::path::Path;
use std::env;

/// Substitute environment variables in a string
/// 
/// Replaces ${VAR_NAME} with the value of the environment variable.
/// If the variable is not set, leaves the placeholder as-is.
fn substitute_env_vars(input: &str) -> String {
    let mut result = input.to_string();
    let mut start = 0;
    
    while let Some(begin) = result[start..].find("${") {
        let begin = start + begin;
        if let Some(end) = result[begin..].find('}') {
            let end = begin + end;
            let var_name = &result[begin + 2..end];
            if let Ok(value) = env::var(var_name) {
                result = format!("{}{}{}", &result[..begin], value, &result[end + 1..]);
                start = begin + value.len();
            } else {
                // Variable not found, skip past this placeholder
                start = end + 1;
            }
        } else {
            break;
        }
    }
    
    result
}

/// Load configuration from a YAML file
pub fn load_config(path: &Path) -> Result<DriftConfig> {
    let contents = std::fs::read_to_string(path)?;
    let contents = substitute_env_vars(&contents);
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

    #[test]
    fn test_substitute_env_vars() {
        env::set_var("DRIFT_TEST_VAR", "hello");
        
        let input = "value: ${DRIFT_TEST_VAR}";
        let result = substitute_env_vars(input);
        assert_eq!(result, "value: hello");
        
        env::remove_var("DRIFT_TEST_VAR");
    }

    #[test]
    fn test_substitute_env_vars_multiple() {
        env::set_var("DRIFT_A", "first");
        env::set_var("DRIFT_B", "second");
        
        let input = "a: ${DRIFT_A}, b: ${DRIFT_B}";
        let result = substitute_env_vars(input);
        assert_eq!(result, "a: first, b: second");
        
        env::remove_var("DRIFT_A");
        env::remove_var("DRIFT_B");
    }

    #[test]
    fn test_substitute_env_vars_missing() {
        // Unset variable should leave placeholder as-is
        let input = "value: ${DRIFT_NONEXISTENT_VAR_12345}";
        let result = substitute_env_vars(input);
        assert_eq!(result, "value: ${DRIFT_NONEXISTENT_VAR_12345}");
    }

    #[test]
    fn test_substitute_env_vars_no_placeholders() {
        let input = "plain text without variables";
        let result = substitute_env_vars(input);
        assert_eq!(result, input);
    }
}
