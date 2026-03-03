//! Drift - Generative ambient music from data streams
//!
//! Transforms data into ambient soundscapes. Weather becomes drones,
//! system metrics become texture, events become percussion.

pub mod config;
pub mod sources;
pub mod mapping;
pub mod synth;
pub mod engine;
pub mod presets;
pub mod viz;

pub use config::DriftConfig;
pub use engine::{Engine, Player};
pub use presets::Preset;
pub use viz::{VizState, SampleBuffer};
