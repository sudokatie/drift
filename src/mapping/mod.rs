//! Mapping system for transforming data to audio parameters
//!
//! Maps data values to audio parameters using various scaling functions.

mod mapper;
mod linear;
mod logarithmic;
mod quantize;
mod threshold;

pub use mapper::{Mapper, MappingPipeline};
pub use linear::LinearMapper;
pub use logarithmic::LogarithmicMapper;
pub use quantize::{QuantizeMapper, Scale};
pub use threshold::{ThresholdMapper, ThresholdDirection, EdgeThresholdMapper};
