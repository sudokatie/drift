//! CLI interface for Drift

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Generative ambient music from data streams
#[derive(Parser)]
#[command(name = "drift")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Play generative ambient music
    Play {
        /// Configuration file path
        #[arg(short, long, default_value = "drift.yaml")]
        config: PathBuf,

        /// Output MIDI instead of audio
        #[arg(long)]
        midi: bool,

        /// MIDI output port name (substring match)
        #[arg(long)]
        midi_port: Option<String>,

        /// MIDI channel (0-15)
        #[arg(long, default_value = "0")]
        midi_channel: u8,
    },

    /// Record to a WAV file
    Record {
        /// Configuration file path
        #[arg(short, long, default_value = "drift.yaml")]
        config: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,
    },

    /// List available audio devices
    Devices,

    /// List available MIDI output ports
    MidiPorts,

    /// Monitor data sources (coming soon)
    Monitor {
        /// Configuration file path
        #[arg(short, long, default_value = "drift.yaml")]
        config: PathBuf,
    },

    /// Validate a configuration file
    Check {
        /// Configuration file path
        #[arg(short, long, default_value = "drift.yaml")]
        config: PathBuf,
    },

    /// Generate an example configuration file
    Init,
}
