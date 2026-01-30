//! Synthesis engine for generating audio
//!
//! Contains oscillators, filters, envelopes, and voice implementations.

mod oscillator;
mod voice;
mod drone;
mod envelope;
mod filter;

pub use oscillator::{Oscillator, Waveform};
pub use voice::Voice;
pub use drone::DroneVoice;
pub use envelope::{Envelope, EnvelopeStage};
pub use filter::{Filter, FilterType};
