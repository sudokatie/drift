//! OSC (Open Sound Control) output
//!
//! Send drift parameters to external software via OSC.

use anyhow::{Context, Result};
use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;

/// OSC output client
pub struct OscOutput {
    socket: UdpSocket,
    target: String,
    prefix: String,
}

/// OSC output configuration
#[derive(Debug, Clone)]
pub struct OscConfig {
    /// Target host (default: 127.0.0.1)
    pub host: String,
    /// Target port (default: 9000)
    pub port: u16,
    /// Address prefix (default: /drift)
    pub prefix: String,
    /// Whether OSC output is enabled
    pub enabled: bool,
}

impl Default for OscConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 9000,
            prefix: "/drift".to_string(),
            enabled: false,
        }
    }
}

impl OscOutput {
    /// Create a new OSC output client
    ///
    /// # Arguments
    /// * `config` - OSC configuration
    pub fn new(config: &OscConfig) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .context("failed to bind OSC socket")?;
        
        // Set non-blocking so sends don't block audio
        socket.set_nonblocking(true)
            .context("failed to set non-blocking")?;

        let target = format!("{}:{}", config.host, config.port);

        Ok(Self {
            socket,
            target,
            prefix: config.prefix.clone(),
        })
    }

    /// Get the target address
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Get the address prefix
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Send a float value to an OSC address
    ///
    /// # Arguments
    /// * `address` - OSC address (will be prefixed with config prefix)
    /// * `value` - Float value to send
    pub fn send_float(&self, address: &str, value: f32) -> Result<()> {
        let full_address = format!("{}{}", self.prefix, address);
        let msg = OscMessage {
            addr: full_address,
            args: vec![OscType::Float(value)],
        };
        self.send_message(msg)
    }

    /// Send an integer value to an OSC address
    ///
    /// # Arguments
    /// * `address` - OSC address (will be prefixed with config prefix)
    /// * `value` - Integer value to send
    pub fn send_int(&self, address: &str, value: i32) -> Result<()> {
        let full_address = format!("{}{}", self.prefix, address);
        let msg = OscMessage {
            addr: full_address,
            args: vec![OscType::Int(value)],
        };
        self.send_message(msg)
    }

    /// Send multiple float values to an OSC address
    ///
    /// # Arguments
    /// * `address` - OSC address (will be prefixed with config prefix)
    /// * `values` - Float values to send
    pub fn send_floats(&self, address: &str, values: &[f32]) -> Result<()> {
        let full_address = format!("{}{}", self.prefix, address);
        let args: Vec<OscType> = values.iter().map(|&v| OscType::Float(v)).collect();
        let msg = OscMessage {
            addr: full_address,
            args,
        };
        self.send_message(msg)
    }

    /// Send a trigger (bang) to an OSC address
    ///
    /// # Arguments
    /// * `address` - OSC address (will be prefixed with config prefix)
    pub fn send_trigger(&self, address: &str) -> Result<()> {
        let full_address = format!("{}{}", self.prefix, address);
        let msg = OscMessage {
            addr: full_address,
            args: vec![],
        };
        self.send_message(msg)
    }

    /// Send a raw OSC message
    fn send_message(&self, msg: OscMessage) -> Result<()> {
        let packet = OscPacket::Message(msg);
        let buf = rosc::encoder::encode(&packet)
            .context("failed to encode OSC message")?;
        
        // Non-blocking send, ignore WouldBlock errors
        match self.socket.send_to(&buf, &self.target) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
            Err(e) => Err(e).context("failed to send OSC message"),
        }
    }
}

/// OSC address mappings for drift parameters
pub struct OscMappings {
    /// Output for sending
    output: OscOutput,
    /// Send rate limit (samples between sends)
    send_interval: usize,
    /// Counter for rate limiting
    counter: usize,
}

impl OscMappings {
    /// Create new OSC mappings
    ///
    /// # Arguments
    /// * `config` - OSC configuration
    /// * `sample_rate` - Audio sample rate
    /// * `updates_per_second` - How many OSC updates per second (default: 60)
    pub fn new(config: &OscConfig, sample_rate: u32, updates_per_second: u32) -> Result<Self> {
        let output = OscOutput::new(config)?;
        let send_interval = (sample_rate / updates_per_second) as usize;

        Ok(Self {
            output,
            send_interval,
            counter: 0,
        })
    }

    /// Update with current engine state
    ///
    /// Call this every sample; it rate-limits internally.
    pub fn update(&mut self, amplitude: f32, pitch: f32, filter_cutoff: f32) -> Result<()> {
        self.counter += 1;
        
        if self.counter >= self.send_interval {
            self.counter = 0;
            
            // Send current values
            self.output.send_float("/amplitude", amplitude)?;
            self.output.send_float("/pitch", pitch)?;
            self.output.send_float("/filter", filter_cutoff)?;
        }
        
        Ok(())
    }

    /// Send a custom parameter
    pub fn send_param(&self, name: &str, value: f32) -> Result<()> {
        let address = format!("/param/{}", name);
        self.output.send_float(&address, value)
    }

    /// Send data source values
    pub fn send_data(&self, source: &str, values: &[f32]) -> Result<()> {
        let address = format!("/data/{}", source);
        self.output.send_floats(&address, values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_config_default() {
        let config = OscConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 9000);
        assert_eq!(config.prefix, "/drift");
        assert!(!config.enabled);
    }

    #[test]
    fn test_osc_output_creation() {
        let config = OscConfig::default();
        let output = OscOutput::new(&config).unwrap();
        assert_eq!(output.target(), "127.0.0.1:9000");
        assert_eq!(output.prefix(), "/drift");
    }

    #[test]
    fn test_osc_output_custom_config() {
        let config = OscConfig {
            host: "192.168.1.100".to_string(),
            port: 8000,
            prefix: "/app".to_string(),
            enabled: true,
        };
        let output = OscOutput::new(&config).unwrap();
        assert_eq!(output.target(), "192.168.1.100:8000");
        assert_eq!(output.prefix(), "/app");
    }

    #[test]
    fn test_osc_send_float() {
        let config = OscConfig::default();
        let output = OscOutput::new(&config).unwrap();
        // Just verify it doesn't error (no receiver)
        let result = output.send_float("/test", 0.5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_osc_send_int() {
        let config = OscConfig::default();
        let output = OscOutput::new(&config).unwrap();
        let result = output.send_int("/test", 42);
        assert!(result.is_ok());
    }

    #[test]
    fn test_osc_send_floats() {
        let config = OscConfig::default();
        let output = OscOutput::new(&config).unwrap();
        let result = output.send_floats("/test", &[0.1, 0.2, 0.3]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_osc_send_trigger() {
        let config = OscConfig::default();
        let output = OscOutput::new(&config).unwrap();
        let result = output.send_trigger("/bang");
        assert!(result.is_ok());
    }

    #[test]
    fn test_osc_mappings_creation() {
        let config = OscConfig::default();
        let mappings = OscMappings::new(&config, 44100, 60).unwrap();
        // send_interval should be 44100 / 60 = 735
        assert_eq!(mappings.send_interval, 735);
    }

    #[test]
    fn test_osc_mappings_rate_limiting() {
        let config = OscConfig::default();
        let mut mappings = OscMappings::new(&config, 44100, 60).unwrap();
        
        // Call update many times
        for _ in 0..1000 {
            let result = mappings.update(0.5, 440.0, 1000.0);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_osc_mappings_send_param() {
        let config = OscConfig::default();
        let mappings = OscMappings::new(&config, 44100, 60).unwrap();
        let result = mappings.send_param("custom", 0.75);
        assert!(result.is_ok());
    }

    #[test]
    fn test_osc_mappings_send_data() {
        let config = OscConfig::default();
        let mappings = OscMappings::new(&config, 44100, 60).unwrap();
        let result = mappings.send_data("cpu", &[0.3, 0.4, 0.5]);
        assert!(result.is_ok());
    }
}
