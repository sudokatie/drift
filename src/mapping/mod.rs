//! Mapping system for transforming data to audio parameters
//!
//! Maps data values to audio parameters using various scaling functions.

mod mapper;
mod linear;
mod quantize;

pub use mapper::{Mapper, MappingPipeline};
pub use linear::LinearMapper;
pub use quantize::{QuantizeMapper, Scale};
