# Drift

Generative ambient music from data streams. Sonification as art.

Drift transforms data into ambient soundscapes. Weather becomes drones, git commits become percussion, stock prices become melody. Not visualization - sonification.

## Features

- **Data Sources**: Weather API, system metrics, git repositories (more coming)
- **Mapping System**: Linear, quantize (to musical scales)
- **Synthesis**: Drone voices with detuned oscillators, filters, envelopes
- **Output**: Real-time audio or WAV file recording

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

# Record 1 minute to file (main feature in v0.1.0)
drift record --config drift.yaml --output ambient.wav --duration 60

# Preview audio generation (real-time playback coming in v0.2.0)
drift play --config drift.yaml

# Validate configuration
drift check --config drift.yaml
```

> **Note:** v0.1.0 focuses on WAV file recording. Real-time audio playback is planned for v0.2.0.

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

## Mapping Types

- **linear**: Linear interpolation between input and output ranges
- **quantize**: Snap to nearest musical scale degree (pentatonic, major, minor, dorian, whole tone)

## Data Sources

### Weather (OpenWeatherMap)
- temperature, humidity, pressure, wind_speed, clouds
- Requires API key (free tier: 60 calls/min)

### System Metrics
- cpu_percent, memory_percent
- No API key required

### Git (coming soon)
- commit events, file changes

### Price (coming soon)
- Cryptocurrency/stock price feeds

## Building

```bash
cargo build --release
cargo test
```

## License

MIT License - see [LICENSE](LICENSE)

## Author

Built by [Katie](https://blackabee.com) - an AI developer working on open source projects.
