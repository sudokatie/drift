# Changelog

All notable changes to Drift will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-30

### Added
- Initial release
- Configuration system with YAML support
- Data sources: Weather (OpenWeatherMap), System metrics
- Mapping system: Linear, Quantize (musical scales)
- Synthesis: Oscillators (sine, triangle, saw, square), Filters (low-pass, high-pass), ADSR envelopes
- Drone voice with detuned oscillators
- Mixer for source-to-voice routing
- CLI commands: play, record, devices, monitor, check, init
- WAV file recording support

### Data Sources
- WeatherSource: Polls OpenWeatherMap API for temperature, humidity, pressure, wind, clouds
- SystemSource: Collects CPU and memory usage via sysinfo

### Voices
- DroneVoice: Sustained tones with multiple detuned oscillators and filtering

### Mapping
- LinearMapper: Linear interpolation with configurable ranges
- QuantizeMapper: Snap frequencies to musical scales (pentatonic, major, minor, etc.)

## [0.1.2] - 2026-01-31

### Fixed
- README Features section no longer claims logarithmic mapper exists
- Unimplemented mapping types (logarithmic, exponential, threshold) now print warning and fall back to linear
- Unimplemented voice types (percussion, melody, texture) now print warning and fall back to drone

## [0.1.1] - 2026-01-31

### Added
- Environment variable substitution in config files (`${VAR_NAME}` syntax)
- 4 new tests for env var substitution

### Fixed
- README no longer claims logarithmic/threshold mappers exist (they don't yet)
- CLI help text clarifies which commands are fully implemented
- Play command now suggests using record command for audio generation

### Documentation
- Added note that v0.1.0 focuses on WAV recording, real-time playback in v0.2.0

## [Unreleased]

### Planned
- Git source for repository events
- Price source for market data
- Real-time audio output via cpal
- TUI mode for live visualization
- Additional voice types (percussion, melody, texture)
- Logarithmic and threshold mappers
- MIDI output support
