//! Data sources for Drift
//!
//! Sources collect data from various inputs (weather, system, etc.)
//! and emit DataPoints for the mapping system.

mod source;
mod system;
mod weather;

pub use source::{DataPoint, Source};
pub use system::SystemSource;
pub use weather::{WeatherConfig, WeatherSource};
