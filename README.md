# Drift

Generative ambient music from data streams. Sonification as art.

Drift transforms data into ambient soundscapes. Weather becomes drones, git commits become percussion, stock prices become melody. Not visualization - sonification.

## Features

- **Data Sources**: Weather API, system metrics, git repositories (more coming)
- **Mapping System**: Linear, logarithmic, quantize (to musical scales)
- **Synthesis**: Drone voices with detuned oscillators, filters, envelopes
- **Output**: Real-time audio or WAV file recording

## Installation

```bash
# From source
git clone https://github.com/katieblackabee/drift
cd drift
cargo build --release
```

## Quick Start

```bash
# Create example config
drift init

# Play (requires data source API keys)
drift play --config drift.yaml

# Record 1 minute to file
drift record --config drift.yaml --output ambient.wav --duration 60

# List audio devices
drift devices

# Validate configuration
drift check --config drift.yaml
```

## Configuration

Drift uses YAML configuration files. See `examples/` for sample configs.

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
- **quantize**: Snap to nearest musical scale degree (pentatonic, major, minor, etc.)
- **logarithmic**: Logarithmic scaling (for frequency/volume)
- **threshold**: Trigger on/off based on threshold

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
