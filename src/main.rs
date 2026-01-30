//! Drift - Generative ambient music from data streams

use clap::Parser;
use drift::config;
use drift::engine::Engine;

mod cli;

use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Play { config: config_path } => {
            println!("Loading configuration from {:?}...", config_path);
            let cfg = config::load_config(&config_path)?;
            
            println!("Starting Drift...");
            println!("  Sample rate: {} Hz", cfg.audio.sample_rate);
            println!("  Master volume: {:.0}%", cfg.master.volume * 100.0);
            
            let mut engine = Engine::new(cfg);
            let drone_idx = engine.add_drone();
            
            // Set initial pitch
            engine.set_voice_parameter(drone_idx, "pitch", 220.0);
            
            println!("\nGenerating audio...");
            println!("(This is a stub - full audio output not yet implemented)");
            
            // Generate a few samples to show it works
            for i in 0..5 {
                let sample = engine.process();
                println!("  Sample {}: {:.6}", i, sample);
            }
            
            println!("\nDrift would be playing ambient music here.");
            println!("Press Ctrl+C to stop (when fully implemented).");
        }
        
        Commands::Record { config: config_path, output, duration } => {
            println!("Loading configuration from {:?}...", config_path);
            let cfg = config::load_config(&config_path)?;
            
            println!("Recording {} seconds to {:?}...", duration, output);
            
            let mut engine = Engine::new(cfg.clone());
            engine.add_drone();
            
            let sample_rate = cfg.audio.sample_rate;
            let total_samples = (sample_rate as u64 * duration) as usize;
            
            // Create WAV writer
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };
            
            let mut writer = hound::WavWriter::create(&output, spec)?;
            
            for i in 0..total_samples {
                let sample = engine.process() as f32;
                writer.write_sample(sample)?;
                
                // Progress update every second
                if i % (sample_rate as usize) == 0 {
                    print!("\r  Progress: {}s / {}s", i / sample_rate as usize, duration);
                    use std::io::Write;
                    std::io::stdout().flush()?;
                }
            }
            
            writer.finalize()?;
            println!("\nRecorded to {:?}", output);
        }
        
        Commands::Devices => {
            println!("Available audio devices:");
            println!("  (Device enumeration not yet implemented)");
            println!("  Use system default device.");
        }
        
        Commands::Monitor { config: config_path } => {
            println!("Loading configuration from {:?}...", config_path);
            let cfg = config::load_config(&config_path)?;
            
            println!("Monitoring {} sources...", cfg.sources.len());
            for source in &cfg.sources {
                println!("  - {} ({:?})", source.name, source.kind);
            }
            
            println!("\n(Source monitoring not yet implemented)");
        }
        
        Commands::Check { config: config_path } => {
            println!("Checking configuration at {:?}...", config_path);
            
            match config::load_config(&config_path) {
                Ok(cfg) => {
                    println!("Configuration is valid!");
                    println!("  Sample rate: {} Hz", cfg.audio.sample_rate);
                    println!("  Sources: {}", cfg.sources.len());
                    println!("  Layers: {}", cfg.layers.len());
                }
                Err(e) => {
                    println!("Configuration is invalid: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Init => {
            let example_config = r#"# Drift Configuration

audio:
  sample_rate: 44100
  buffer_size: 512

master:
  bpm: 60
  key: C
  scale: minor_pentatonic
  volume: 0.7

sources:
  - name: system
    kind: system
    enabled: true
    settings:
      interval_ms: 1000

layers:
  - name: system_drone
    voice: drone
    source: system
    volume: 0.8
    mappings:
      pitch:
        field: cpu_percent
        kind: linear
        in_min: 0
        in_max: 100
        out_min: 100
        out_max: 400
      filter:
        field: memory_percent
        kind: linear
        in_min: 0
        in_max: 100
        out_min: 200
        out_max: 2000
"#;
            
            let path = "drift.yaml";
            if std::path::Path::new(path).exists() {
                println!("drift.yaml already exists. Not overwriting.");
            } else {
                std::fs::write(path, example_config)?;
                println!("Created drift.yaml with example configuration.");
            }
        }
    }
    
    Ok(())
}
