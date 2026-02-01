//! System metrics source
//!
//! Collects CPU, memory, and other system metrics.

use super::{DataPoint, Source};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Configuration for system source
#[derive(Debug, Clone)]
pub struct SystemConfig {
    /// Poll interval
    pub interval: Duration,
}

impl SystemConfig {
    /// Create config from settings map
    pub fn from_settings(settings: &HashMap<String, serde_yaml::Value>) -> Result<Self> {
        let interval_ms = settings
            .get("interval_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000); // 1 second default

        Ok(Self {
            interval: Duration::from_millis(interval_ms),
        })
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(1),
        }
    }
}

/// Source that collects system metrics
pub struct SystemSource {
    name: String,
    interval: Duration,
    running: Arc<AtomicBool>,
    sender: broadcast::Sender<DataPoint>,
    task: Option<JoinHandle<()>>,
}

impl SystemSource {
    /// Create a new system source with default interval (1 second)
    pub fn new(name: impl Into<String>) -> Self {
        Self::with_interval(name, Duration::from_secs(1))
    }

    /// Create a new system source with custom interval
    pub fn with_interval(name: impl Into<String>, interval: Duration) -> Self {
        let (sender, _) = broadcast::channel(16);
        Self {
            name: name.into(),
            interval,
            running: Arc::new(AtomicBool::new(false)),
            sender,
            task: None,
        }
    }

    /// Create a new system source with config
    pub fn with_config(name: impl Into<String>, config: SystemConfig) -> Self {
        Self::with_interval(name, config.interval)
    }
}

impl Source for SystemSource {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn start(&mut self) -> anyhow::Result<()> {
        if self.is_running() {
            return Ok(());
        }
        
        self.running.store(true, Ordering::SeqCst);
        
        let name = self.name.clone();
        let interval = self.interval;
        let running = Arc::clone(&self.running);
        let sender = self.sender.clone();
        
        let task = tokio::spawn(async move {
            let mut sys = System::new_all();
            
            while running.load(Ordering::SeqCst) {
                sys.refresh_all();
                
                // Calculate CPU usage
                let cpu_usage = sys.global_cpu_usage() as f64;
                
                // Calculate memory usage
                let total_memory = sys.total_memory() as f64;
                let used_memory = sys.used_memory() as f64;
                let memory_percent = if total_memory > 0.0 {
                    (used_memory / total_memory) * 100.0
                } else {
                    0.0
                };
                
                // Create data point
                let point = DataPoint::new(&name)
                    .with_value("cpu_percent", cpu_usage)
                    .with_value("memory_percent", memory_percent)
                    .with_value("memory_used_bytes", used_memory)
                    .with_value("memory_total_bytes", total_memory);
                
                // Send (ignore errors if no receivers)
                let _ = sender.send(point);
                
                tokio::time::sleep(interval).await;
            }
        });
        
        self.task = Some(task);
        Ok(())
    }
    
    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
    
    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    fn subscribe(&self) -> broadcast::Receiver<DataPoint> {
        self.sender.subscribe()
    }
}

impl Drop for SystemSource {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_config_from_settings() {
        let mut settings = HashMap::new();
        settings.insert(
            "interval_ms".to_string(),
            serde_yaml::Value::Number(500.into()),
        );

        let config = SystemConfig::from_settings(&settings).unwrap();
        assert_eq!(config.interval, Duration::from_millis(500));
    }

    #[test]
    fn test_system_config_defaults() {
        let settings = HashMap::new();
        let config = SystemConfig::from_settings(&settings).unwrap();
        assert_eq!(config.interval, Duration::from_millis(1000));
    }

    #[tokio::test]
    async fn test_system_source_creation() {
        let source = SystemSource::new("test_system");
        assert_eq!(source.name(), "test_system");
        assert!(!source.is_running());
    }

    #[tokio::test]
    async fn test_system_source_with_interval() {
        let source = SystemSource::with_interval("test_system", Duration::from_millis(500));
        assert_eq!(source.name(), "test_system");
        assert!(!source.is_running());
    }

    #[tokio::test]
    async fn test_system_source_start_stop() {
        let mut source = SystemSource::with_interval("test_system", Duration::from_millis(100));

        source.start().unwrap();
        assert!(source.is_running());

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(50)).await;

        source.stop();
        assert!(!source.is_running());
    }

    #[tokio::test]
    async fn test_system_source_data() {
        let mut source = SystemSource::with_interval("test_system", Duration::from_millis(100));
        let mut receiver = source.subscribe();

        source.start().unwrap();

        // Wait for a data point
        let result =
            tokio::time::timeout(Duration::from_secs(2), receiver.recv()).await;

        source.stop();

        let point = result.expect("timeout").expect("receive error");
        assert_eq!(point.source, "test_system");
        assert!(point.values.contains_key("cpu_percent"));
        assert!(point.values.contains_key("memory_percent"));
    }
}
