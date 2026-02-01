//! Drift - Generative ambient music from data streams

use anyhow::Result;
use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use drift::config::{self, SourceKind};
use drift::engine::{Engine, Recorder};
use drift::sources::{GitConfig, GitSource, PriceConfig, PriceSource, Source, SystemSource, WeatherConfig, WeatherSource};

mod cli;

use cli::{Cli, Commands};

fn main() -> Result<()> {
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

            println!("\nAudio preview (real-time playback coming in v0.2.0):");

            // Generate a few samples to show it works
            for i in 0..5 {
                let sample = engine.process();
                println!("  Sample {}: {:.6}", i, sample);
            }

            println!("\nTo generate audio now, use the record command:");
            println!(
                "  drift record --config {:?} --output ambient.wav --duration 60",
                config_path
            );
        }

        Commands::Record {
            config: config_path,
            output,
            duration,
        } => {
            println!("Loading configuration from {:?}...", config_path);
            let cfg = config::load_config(&config_path)?;

            println!("Recording {} seconds to {:?}...", duration, output);

            let mut engine = Engine::new(cfg.clone());
            engine.add_drone();

            let sample_rate = cfg.audio.sample_rate;
            let total_samples = (sample_rate as u64 * duration) as usize;

            // Create recorder
            let mut recorder = Recorder::new(&output, sample_rate)?;

            for i in 0..total_samples {
                let sample = engine.process() as f32;
                recorder.write_sample(sample)?;

                // Progress update every second
                if i % (sample_rate as usize) == 0 {
                    print!(
                        "\r  Progress: {}s / {}s",
                        i / sample_rate as usize,
                        duration
                    );
                    use std::io::Write;
                    std::io::stdout().flush()?;
                }
            }

            recorder.finalize()?;
            println!("\nRecorded to {:?}", output);
        }

        Commands::Devices => {
            println!("Available audio devices:\n");

            let host = cpal::default_host();

            // Default output device
            if let Some(device) = host.default_output_device() {
                println!("Default output: {}", device.name().unwrap_or_default());
                if let Ok(config) = device.default_output_config() {
                    println!(
                        "  Sample rate: {} Hz, Channels: {}",
                        config.sample_rate().0,
                        config.channels()
                    );
                }
                println!();
            }

            // List all output devices
            println!("Output devices:");
            match host.output_devices() {
                Ok(devices) => {
                    for device in devices {
                        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                        print!("  - {}", name);

                        if let Ok(config) = device.default_output_config() {
                            print!(
                                " ({} Hz, {} ch)",
                                config.sample_rate().0,
                                config.channels()
                            );
                        }
                        println!();
                    }
                }
                Err(e) => {
                    println!("  Error listing devices: {}", e);
                }
            }

            // List all input devices
            println!("\nInput devices:");
            match host.input_devices() {
                Ok(devices) => {
                    for device in devices {
                        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                        print!("  - {}", name);

                        if let Ok(config) = device.default_input_config() {
                            print!(
                                " ({} Hz, {} ch)",
                                config.sample_rate().0,
                                config.channels()
                            );
                        }
                        println!();
                    }
                }
                Err(e) => {
                    println!("  Error listing devices: {}", e);
                }
            }
        }

        Commands::Monitor { config: config_path } => {
            println!("Loading configuration from {:?}...", config_path);
            let cfg = config::load_config(&config_path)?;

            if cfg.sources.is_empty() {
                println!("No sources configured.");
                return Ok(());
            }

            println!("Monitoring {} sources...\n", cfg.sources.len());

            // Create and start sources
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                for source_config in &cfg.sources {
                    if !source_config.enabled {
                        println!("{}: disabled", source_config.name);
                        continue;
                    }

                    println!("{}:", source_config.name);
                    println!("  Type: {:?}", source_config.kind);

                    match source_config.kind {
                        SourceKind::Weather => {
                            match WeatherConfig::from_settings(&source_config.settings) {
                                Ok(wc) => {
                                    println!("  Location: {}", wc.location);
                                    println!("  Interval: {:?}", wc.interval);
                                    println!("  API key: {}...", &wc.api_key.chars().take(8).collect::<String>());
                                    
                                    // Try to fetch current data
                                    let mut source = WeatherSource::new(&source_config.name, wc);
                                    let mut rx = source.subscribe();
                                    source.start().ok();
                                    
                                    // Wait briefly for data
                                    tokio::select! {
                                        Ok(data) = rx.recv() => {
                                            println!("  Current readings:");
                                            for (key, value) in &data.values {
                                                println!("    {}: {:.2}", key, value);
                                            }
                                        }
                                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                                            println!("  (Timeout waiting for data)");
                                        }
                                    }
                                    source.stop();
                                }
                                Err(e) => println!("  Error: {}", e),
                            }
                        }
                        SourceKind::System => {
                            let mut source = SystemSource::new(&source_config.name);
                            let mut rx = source.subscribe();
                            source.start().ok();
                            
                            tokio::select! {
                                Ok(data) = rx.recv() => {
                                    println!("  Current readings:");
                                    for (key, value) in &data.values {
                                        println!("    {}: {:.2}", key, value);
                                    }
                                }
                                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                                    println!("  (Timeout waiting for data)");
                                }
                            }
                            source.stop();
                        }
                        SourceKind::Git => {
                            match GitConfig::from_settings(&source_config.settings) {
                                Ok(gc) => {
                                    println!("  Path: {:?}", gc.path);
                                    println!("  Interval: {:?}", gc.interval);
                                    
                                    let mut source = GitSource::new(&source_config.name, gc);
                                    let mut rx = source.subscribe();
                                    if source.start().is_ok() {
                                        tokio::select! {
                                            Ok(data) = rx.recv() => {
                                                println!("  Current readings:");
                                                for (key, value) in &data.values {
                                                    println!("    {}: {:.2}", key, value);
                                                }
                                            }
                                            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {
                                                println!("  (Timeout waiting for data)");
                                            }
                                        }
                                        source.stop();
                                    } else {
                                        println!("  (Failed to start - check path)");
                                    }
                                }
                                Err(e) => println!("  Error: {}", e),
                            }
                        }
                        SourceKind::Price => {
                            match PriceConfig::from_settings(&source_config.settings) {
                                Ok(pc) => {
                                    println!("  Symbols: {:?}", pc.symbols);
                                    println!("  Interval: {:?}", pc.interval);
                                    
                                    let mut source = PriceSource::new(&source_config.name, pc);
                                    let mut rx = source.subscribe();
                                    source.start().ok();
                                    
                                    tokio::select! {
                                        Ok(data) = rx.recv() => {
                                            println!("  Current readings:");
                                            for (key, value) in &data.values {
                                                println!("    {}: {:.2}", key, value);
                                            }
                                        }
                                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                                            println!("  (Timeout waiting for data)");
                                        }
                                    }
                                    source.stop();
                                }
                                Err(e) => println!("  Error: {}", e),
                            }
                        }
                    }
                    println!();
                }
            });
        }

        Commands::Check { config: config_path } => {
            println!("Checking configuration at {:?}...", config_path);

            match config::load_config(&config_path) {
                Ok(cfg) => {
                    println!("Configuration is valid!");
                    println!("  Sample rate: {} Hz", cfg.audio.sample_rate);
                    println!("  Buffer size: {}", cfg.audio.buffer_size);
                    println!("  Master volume: {:.0}%", cfg.master.volume * 100.0);
                    println!("  BPM: {}", cfg.master.bpm);
                    println!("  Key: {}", cfg.master.key);
                    println!("  Scale: {}", cfg.master.scale);
                    println!("  Sources: {}", cfg.sources.len());
                    for source in &cfg.sources {
                        println!(
                            "    - {} ({:?}) {}",
                            source.name,
                            source.kind,
                            if source.enabled { "[enabled]" } else { "[disabled]" }
                        );
                    }
                    println!("  Layers: {}", cfg.layers.len());
                    for layer in &cfg.layers {
                        println!(
                            "    - {} ({:?}) -> {}",
                            layer.name, layer.voice, layer.source
                        );
                    }
                }
                Err(e) => {
                    println!("Configuration is invalid: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Init => {
            let example_config = include_str!("../drift.example.yaml");

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
