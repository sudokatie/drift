//! Source trait and DataPoint definition

use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::broadcast;

/// A data point emitted by a source
#[derive(Debug, Clone)]
pub struct DataPoint {
    /// Name of the source that emitted this
    pub source: String,
    
    /// When this data was collected
    pub timestamp: Instant,
    
    /// Numeric values (e.g., temperature: 22.5)
    pub values: HashMap<String, f64>,
    
    /// Discrete events (e.g., "commit", "high_wind")
    pub events: Vec<String>,
}

impl DataPoint {
    /// Create a new data point
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            timestamp: Instant::now(),
            values: HashMap::new(),
            events: Vec::new(),
        }
    }
    
    /// Add a numeric value
    pub fn with_value(mut self, key: impl Into<String>, value: f64) -> Self {
        self.values.insert(key.into(), value);
        self
    }
    
    /// Add an event
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.events.push(event.into());
        self
    }
}

/// Trait for data sources
pub trait Source: Send + Sync {
    /// Get the name of this source
    fn name(&self) -> &str;
    
    /// Start collecting data
    fn start(&mut self) -> anyhow::Result<()>;
    
    /// Stop collecting data
    fn stop(&mut self);
    
    /// Check if the source is running
    fn is_running(&self) -> bool;
    
    /// Subscribe to data points from this source
    fn subscribe(&self) -> broadcast::Receiver<DataPoint>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_point_creation() {
        let point = DataPoint::new("test")
            .with_value("temperature", 22.5)
            .with_value("humidity", 65.0)
            .with_event("update");
        
        assert_eq!(point.source, "test");
        assert_eq!(point.values.get("temperature"), Some(&22.5));
        assert_eq!(point.values.get("humidity"), Some(&65.0));
        assert_eq!(point.events.len(), 1);
        assert_eq!(point.events[0], "update");
    }
}
