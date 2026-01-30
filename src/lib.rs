//! Drift - Generative ambient music from data streams
//!
//! Transforms data into ambient soundscapes. Weather becomes drones,
//! system metrics become texture, events become percussion.

pub mod config;
pub mod sources;
pub mod mapping;
pub mod synth;
pub mod engine;

pub use config::DriftConfig;
pub use engine::Engine;
