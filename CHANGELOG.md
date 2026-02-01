# Changelog

All notable changes to Drift will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Real-time audio output via cpal
- TUI mode for live visualization
- Additional voice types (percussion, melody, texture)
- MIDI output support

## [0.1.3] - 2026-02-01

### Added
- **Git source**: Watch repositories for commits, branch changes, file modifications
  - Tracks commit_count, modified_count, staged_count, activity
  - Events: commit, branch_change, staged, file_change
- **Price source**: Cryptocurrency prices from CoinGecko API
  - Tracks price, volume, change_24h, volatility
  - Events: pump (>5% up), dump (>5% down)
- **Pattern mapper**: Euclidean rhythm generator for converting data to rhythmic triggers
  - Implements Bjorklund's algorithm
  - Supports common world music rhythms (E(3,8), E(5,8), etc.)
- **Recorder module**: Extracted WAV recording into reusable engine component
- **drift.example.yaml**: Root-level example config file
- **Full device enumeration**: `drift devices` now lists all audio input/output devices
- **Full monitor command**: `drift monitor` now fetches and displays real data from sources

### Changed
- README now accurately reflects all implemented features
- CLI uses proper Recorder module instead of inline hound calls
- init command now uses drift.example.yaml as template

### Tests
- 141 unit tests (up from 96)
- 1 doc test
- All tests passing
- Clippy clean

## [0.1.2] - 2026-01-31

### Added
- **Noise oscillators**: White, pink, and brown noise waveforms
- **LFO modulation**: Low Frequency Oscillator for filter and pitch modulation
- **Logarithmic mapper**: Perceptually linear scaling for frequency/volume
- **Threshold mapper**: Binary trigger for percussion and event detection
- **Edge threshold mapper**: Stateful edge detection for crossing events

### Changed
- DroneVoice now uses ADSR envelope (configurable attack, decay, sustain, release)
- DroneVoice now uses biquad filter instead of simple one-pole
- DroneVoice includes filter LFO modulation and pitch vibrato
- DroneVoice now mixes sub oscillator and pink noise for richer sound
- All mapping types now properly implemented (no more fallback warnings)

### Fixed
- Test updated to properly handle ADSR release timing
- Removed dead code warnings in threshold mapper

## [0.1.1] - 2026-01-31

### Added
- Environment variable substitution in config files (`${VAR_NAME}` syntax)
- 4 new tests for env var substitution

### Fixed
- README no longer claims non-existent features
- CLI help text clarifies which commands are fully implemented
- Play command now suggests using record command for audio generation

### Documentation
- Added note that v0.1.0 focuses on WAV recording, real-time playback in v0.2.0

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
