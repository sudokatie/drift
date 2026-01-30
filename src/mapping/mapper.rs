//! Mapper trait and pipeline

/// Trait for mapping functions
pub trait Mapper: Send + Sync {
    /// Get the name of this mapper
    fn name(&self) -> &str;
    
    /// Map an input value to an output value
    fn map(&self, input: f64) -> f64;
}

/// A pipeline of mappers applied in sequence
pub struct MappingPipeline {
    mappers: Vec<Box<dyn Mapper>>,
}

impl MappingPipeline {
    /// Create an empty pipeline
    pub fn new() -> Self {
        Self { mappers: Vec::new() }
    }
    
    /// Add a mapper to the pipeline (builder pattern)
    pub fn with<M: Mapper + 'static>(mut self, mapper: M) -> Self {
        self.mappers.push(Box::new(mapper));
        self
    }
    
    /// Apply all mappers in sequence
    pub fn apply(&self, mut value: f64) -> f64 {
        for mapper in &self.mappers {
            value = mapper.map(value);
        }
        value
    }
    
    /// Check if the pipeline is empty
    pub fn is_empty(&self) -> bool {
        self.mappers.is_empty()
    }
}

impl Default for MappingPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::LinearMapper;

    #[test]
    fn test_pipeline_single_mapper() {
        let pipeline = MappingPipeline::new()
            .with(LinearMapper::new("test", 0.0, 100.0, 0.0, 1.0));
        
        assert!(!pipeline.is_empty());
        assert_eq!(pipeline.apply(0.0), 0.0);
        assert_eq!(pipeline.apply(50.0), 0.5);
        assert_eq!(pipeline.apply(100.0), 1.0);
    }

    #[test]
    fn test_pipeline_chained_mappers() {
        let pipeline = MappingPipeline::new()
            .with(LinearMapper::new("first", 0.0, 100.0, 0.0, 10.0))
            .with(LinearMapper::new("second", 0.0, 10.0, 100.0, 200.0));
        
        // 50 -> 5.0 -> 150.0
        assert_eq!(pipeline.apply(50.0), 150.0);
    }
}
