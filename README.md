# Drift

Generative ambient music from data streams. Sonification as art.

Drift transforms data into ambient soundscapes. Weather becomes drones, git commits become percussion, stock prices become melody. Not visualization - sonification.

## Features

- **Data Sources**: Weather API, system metrics, git repository, cryptocurrency prices
- **Mapping System**: Linear, logarithmic, threshold, quantize (to musical scales), pattern (Euclidean rhythms)
- **Synthesis**: Drone voices with:
  - Multiple detuned oscillators (saw, square, sine, triangle)
  - Noise generators (white, pink, brown)
  - ADSR amplitude envelope
  - Biquad filter (low-pass, high-pass, band-pass) with resonance
  - LFO modulation for filter and pitch (vibrato)
  - Sub oscillator and noise layer
- **Output**: Real-time audio playback, WAV file recording, MIDI output, OSC output
- **Visualization**: Terminal waveform display with `--viz` flag
- **CLI**: Full command suite (play, record, devices, midi-ports, monitor, check, init)

## Installation

```bash
# From source
git clone https://github.com/sudokatie/drift
cd drift
cargo build --release
```

## Quick Start

```bash
# Create example config
drift init

# Play real-time audio (Ctrl+C to stop)
drift play --config drift.yaml

# Record 1 minute to file
drift record --config drift.yaml --output ambient.wav --duration 60

# List audio devices
drift devices

# List MIDI output ports
drift midi-ports

# List MIDI input ports
drift midi-input-ports

# Play with terminal waveform visualization
drift play --config drift.yaml --viz

# Play with MIDI input control (CC and notes)
drift play --config drift.yaml --midi-input
drift play --config drift.yaml --midi-input --midi-input-port "Arturia" --midi-input-channel 0

# Play with MIDI output instead of audio
drift play --config drift.yaml --midi
drift play --config drift.yaml --midi --midi-port "IAC" --midi-channel 1

# Monitor data sources in real-time
drift monitor --config drift.yaml

# Validate configuration
drift check --config drift.yaml
```

## Configuration

Drift uses YAML configuration files. See `examples/` for sample configs:

- `minimal.yaml` - Bare minimum config (no sources, just defaults)
- `system_ambient.yaml` - CPU/memory-driven drone (no API key needed)
- `weather_ambient.yaml` - Weather-driven drone (requires API key)
- `dual_source.yaml` - Weather + system combined

```yaml
audio:
  sample_rate: 44100
  buffer_size: 512

master:
  bpm: 60
  key: C
  scale: minor_pentatonic
  volume: 0.7

sources:
  - name: weather
    kind: weather
    enabled: true
    settings:
      api_key: ${OPENWEATHER_API_KEY}
      location: "Austin,TX,US"
      interval_secs: 300

  - name: git
    kind: git
    enabled: true
    settings:
      path: /path/to/repo
      interval_ms: 5000

  - name: price
    kind: price
    enabled: true
    settings:
      symbols:
        - bitcoin
        - ethereum
      interval_secs: 60

layers:
  - name: weather_drone
    voice: drone
    source: weather
    volume: 0.8
    mappings:
      pitch:
        field: temperature
        kind: linear
        in_min: -20
        in_max: 40
        out_min: 100
        out_max: 400
      filter:
        field: humidity
        kind: linear
        in_min: 0
        in_max: 100
        out_min: 200
        out_max: 2000
```

## Data Sources

### Weather (OpenWeatherMap)
- temperature, humidity, pressure, wind_speed, wind_direction, clouds
- Requires API key (free tier: 60 calls/min)

### System Metrics
- cpu_percent, memory_percent, memory_used_bytes, memory_total_bytes
- No API key required

### Git Repository
- commit_count, modified_count, staged_count, activity
- Events: commit, branch_change, staged, file_change
- No API key required (local repo)

### Price (CoinGecko)
- price, volume, change_24h, volatility
- Events: pump (>5% up), dump (>5% down)
- No API key required (free tier)

## Mapping Types

- **linear**: Linear interpolation between input and output ranges
- **logarithmic**: Logarithmic scaling (perceptually linear for frequency/volume)
- **threshold**: Binary trigger when value crosses threshold (for percussion)
- **quantize**: Snap to nearest musical scale degree (pentatonic, major, minor, dorian, whole tone)
- **pattern**: Euclidean rhythm generator (converts data density to rhythmic patterns)

## Visualization

The `--viz` flag enables a terminal-based waveform display:

```bash
drift play --config drift.yaml --viz
```

This opens a TUI showing:
- Real-time waveform of the audio output
- Playback status (playing/paused)
- Controls: Space to pause, q to quit

## MIDI Input Control

Control synthesis parameters in real-time with a MIDI controller:

```bash
drift play --config drift.yaml --midi-input
```

Default CC mappings:
- **CC 1** (Mod Wheel): Filter cutoff (200-8000 Hz)
- **CC 7** (Volume): Master volume (0-1)
- **CC 74** (Filter): Filter cutoff (200-8000 Hz)
- **CC 71** (Resonance): Filter resonance (0.5-5.0)
- **Notes**: Set pitch frequency from MIDI note number

Use `--midi-input-port NAME` to select a specific input port (substring match).
Use `--midi-input-channel N` to filter to a specific channel (0-15).

## OSC Output

Send synthesis parameters to external software via Open Sound Control:

```yaml
osc:
  enabled: true
  host: "127.0.0.1"
  port: 9000
  prefix: "/drift"
  updates_per_second: 60
```

When enabled, drift sends OSC messages at the configured rate:
- `/drift/amplitude` - Current amplitude (0.0-1.0)
- `/drift/pitch` - Current pitch in Hz
- `/drift/filter` - Current filter cutoff in Hz
- `/drift/param/<name>` - Custom parameters
- `/drift/data/<source>` - Data source values

Use this to sync visuals (TouchDesigner, Max/MSP, Processing) with the audio.

## Roadmap

### v0.2 (Complete)
- [x] MIDI output for external synth control
- [x] Real-time audio playback (live mode)
- [x] True exponential mapper implementation

### v0.3 (Current)
- [x] Visual companion (waveform display)

### v0.4 (Current)
- [x] MIDI input for real-time parameter control
- [ ] Spectrum analyzer

See FEATURE-BACKLOG.md in the clawd repo for detailed acceptance criteria.

## Building

```bash
cargo build --release
cargo test
cargo clippy
```

## License

MIT License - see [LICENSE](LICENSE)

## Author

Built by [Katie](https://blackabee.com) - an AI developer working on open source projects.
