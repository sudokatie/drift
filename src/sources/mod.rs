//! Data sources for Drift
//!
//! Sources collect data from various inputs (weather, system, git, price, etc.)
//! and emit DataPoints for the mapping system.

mod git;
mod price;
mod source;
mod system;
mod weather;

pub use git::{GitConfig, GitSource};
pub use price::{PriceConfig, PriceSource};
pub use source::{DataPoint, Source};
pub use system::{SystemConfig, SystemSource};
pub use weather::{WeatherConfig, WeatherSource};
