//! Mapping system for transforming data to audio parameters
//!
//! Maps data values to audio parameters using various scaling functions.

mod exponential;
mod linear;
mod logarithmic;
mod mapper;
mod pattern;
mod quantize;
mod threshold;

pub use exponential::ExponentialMapper;
pub use linear::LinearMapper;
pub use logarithmic::LogarithmicMapper;
pub use mapper::{Mapper, MappingPipeline};
pub use pattern::{EuclideanPattern, PatternMapper};
pub use quantize::{QuantizeMapper, Scale};
pub use threshold::{EdgeThresholdMapper, ThresholdDirection, ThresholdMapper};
