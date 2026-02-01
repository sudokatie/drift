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
- **Output**: WAV file recording, real-time audio device enumeration
- **CLI**: Full command suite (play, record, devices, monitor, check, init)

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

# Record 1 minute to file
drift record --config drift.yaml --output ambient.wav --duration 60

# Preview audio generation
drift play --config drift.yaml

# List audio devices
drift devices

# Monitor data sources in real-time
drift monitor --config drift.yaml

# Validate configuration
drift check --config drift.yaml
```

## Configuration

Drift uses YAML configuration files. See `examples/` for sample configs:

- `weather_ambient.yaml` - Weather-driven drone (requires API key)
- `system_ambient.yaml` - CPU/memory-driven drone (no API key needed)
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
