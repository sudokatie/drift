//! Weather data source
//!
//! Collects weather data from OpenWeatherMap API and emits DataPoints.

use super::{DataPoint, Source};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// OpenWeatherMap API response
#[derive(Debug, Deserialize)]
struct WeatherResponse {
    main: MainData,
    wind: Option<WindData>,
    clouds: Option<CloudData>,
    weather: Vec<WeatherCondition>,
}

#[derive(Debug, Deserialize)]
struct MainData {
    temp: f64,
    humidity: f64,
    pressure: f64,
    #[serde(default)]
    feels_like: f64,
}

#[derive(Debug, Deserialize)]
struct WindData {
    speed: f64,
    #[serde(default)]
    deg: f64,
    #[serde(default)]
    gust: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CloudData {
    all: f64,
}

#[derive(Debug, Deserialize)]
struct WeatherCondition {
    main: String,
    #[allow(dead_code)]
    description: String,
}

/// Configuration for weather source
#[derive(Debug, Clone)]
pub struct WeatherConfig {
    /// OpenWeatherMap API key
    pub api_key: String,
    /// Location query (city name, "lat,lon", or city ID)
    pub location: String,
    /// Poll interval
    pub interval: Duration,
    /// Use metric units (Celsius). If false, uses Fahrenheit.
    pub metric: bool,
}

impl WeatherConfig {
    /// Create config from settings map
    pub fn from_settings(settings: &HashMap<String, serde_yaml::Value>) -> Result<Self> {
        let api_key = settings
            .get("api_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .context("weather source requires 'api_key' setting")?;
        
        let location = settings
            .get("location")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Austin,TX,US".to_string());
        
        let interval_secs = settings
            .get("interval_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(300); // 5 minutes default
        
        let metric = settings
            .get("metric")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        Ok(Self {
            api_key,
            location,
            interval: Duration::from_secs(interval_secs),
            metric,
        })
    }
}

/// Source that collects weather data from OpenWeatherMap
pub struct WeatherSource {
    name: String,
    config: WeatherConfig,
    running: Arc<AtomicBool>,
    sender: broadcast::Sender<DataPoint>,
    task: Option<JoinHandle<()>>,
}

impl WeatherSource {
    /// Create a new weather source
    pub fn new(name: impl Into<String>, config: WeatherConfig) -> Self {
        let (sender, _) = broadcast::channel(16);
        Self {
            name: name.into(),
            config,
            running: Arc::new(AtomicBool::new(false)),
            sender,
            task: None,
        }
    }
    
    /// Build the API URL
    fn build_url(&self) -> String {
        let units = if self.config.metric { "metric" } else { "imperial" };
        format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units={}",
            urlencoding::encode(&self.config.location),
            self.config.api_key,
            units
        )
    }
    
    /// Fetch weather data from API
    async fn fetch_weather(url: &str) -> Result<WeatherResponse> {
        let response = reqwest::get(url)
            .await
            .context("failed to fetch weather data")?;
        
        if !response.status().is_success() {
            bail!("weather API returned status {}", response.status());
        }
        
        response
            .json::<WeatherResponse>()
            .await
            .context("failed to parse weather response")
    }
    
    /// Convert API response to DataPoint
    fn response_to_datapoint(name: &str, response: &WeatherResponse) -> DataPoint {
        let mut point = DataPoint::new(name)
            .with_value("temperature", response.main.temp)
            .with_value("humidity", response.main.humidity)
            .with_value("pressure", response.main.pressure)
            .with_value("feels_like", response.main.feels_like);
        
        // Add wind data if present
        if let Some(wind) = &response.wind {
            point = point
                .with_value("wind_speed", wind.speed)
                .with_value("wind_direction", wind.deg);
            if let Some(gust) = wind.gust {
                point = point.with_value("wind_gust", gust);
            }
        }
        
        // Add cloud coverage if present
        if let Some(clouds) = &response.clouds {
            point = point.with_value("clouds", clouds.all);
        }
        
        // Add weather condition as event
        if let Some(condition) = response.weather.first() {
            point = point.with_event(&condition.main);
        }
        
        point
    }
}

impl Source for WeatherSource {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }
        
        self.running.store(true, Ordering::SeqCst);
        
        let name = self.name.clone();
        let url = self.build_url();
        let interval = self.config.interval;
        let running = Arc::clone(&self.running);
        let sender = self.sender.clone();
        
        let task = tokio::spawn(async move {
            while running.load(Ordering::SeqCst) {
                match Self::fetch_weather(&url).await {
                    Ok(response) => {
                        let point = Self::response_to_datapoint(&name, &response);
                        let _ = sender.send(point);
                    }
                    Err(e) => {
                        // Log error but keep running
                        eprintln!("Weather fetch error: {}", e);
                    }
                }
                
                tokio::time::sleep(interval).await;
            }
        });
        
        self.task = Some(task);
        Ok(())
    }
    
    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
    
    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    fn subscribe(&self) -> broadcast::Receiver<DataPoint> {
        self.sender.subscribe()
    }
}

impl Drop for WeatherSource {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_config_from_settings() {
        let mut settings = HashMap::new();
        settings.insert("api_key".to_string(), serde_yaml::Value::String("test123".to_string()));
        settings.insert("location".to_string(), serde_yaml::Value::String("London,UK".to_string()));
        settings.insert("interval_secs".to_string(), serde_yaml::Value::Number(600.into()));
        settings.insert("metric".to_string(), serde_yaml::Value::Bool(true));
        
        let config = WeatherConfig::from_settings(&settings).unwrap();
        assert_eq!(config.api_key, "test123");
        assert_eq!(config.location, "London,UK");
        assert_eq!(config.interval, Duration::from_secs(600));
        assert!(config.metric);
    }
    
    #[test]
    fn test_weather_config_defaults() {
        let mut settings = HashMap::new();
        settings.insert("api_key".to_string(), serde_yaml::Value::String("test".to_string()));
        
        let config = WeatherConfig::from_settings(&settings).unwrap();
        assert_eq!(config.location, "Austin,TX,US");
        assert_eq!(config.interval, Duration::from_secs(300));
        assert!(config.metric);
    }
    
    #[test]
    fn test_weather_config_missing_api_key() {
        let settings = HashMap::new();
        let result = WeatherConfig::from_settings(&settings);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_weather_source_creation() {
        let config = WeatherConfig {
            api_key: "test".to_string(),
            location: "Austin,TX,US".to_string(),
            interval: Duration::from_secs(300),
            metric: true,
        };
        let source = WeatherSource::new("test_weather", config);
        assert_eq!(source.name(), "test_weather");
        assert!(!source.is_running());
    }
    
    #[test]
    fn test_build_url() {
        let config = WeatherConfig {
            api_key: "abc123".to_string(),
            location: "Austin,TX".to_string(),
            interval: Duration::from_secs(300),
            metric: true,
        };
        let source = WeatherSource::new("test", config);
        let url = source.build_url();
        assert!(url.contains("api.openweathermap.org"));
        assert!(url.contains("abc123"));
        assert!(url.contains("Austin"));
        assert!(url.contains("metric"));
    }
    
    #[test]
    fn test_build_url_imperial() {
        let config = WeatherConfig {
            api_key: "abc123".to_string(),
            location: "Austin,TX".to_string(),
            interval: Duration::from_secs(300),
            metric: false,
        };
        let source = WeatherSource::new("test", config);
        let url = source.build_url();
        assert!(url.contains("imperial"));
    }
    
    #[test]
    fn test_response_to_datapoint() {
        let response = WeatherResponse {
            main: MainData {
                temp: 22.5,
                humidity: 65.0,
                pressure: 1013.0,
                feels_like: 23.0,
            },
            wind: Some(WindData {
                speed: 3.5,
                deg: 180.0,
                gust: Some(5.0),
            }),
            clouds: Some(CloudData { all: 40.0 }),
            weather: vec![WeatherCondition {
                main: "Clouds".to_string(),
                description: "scattered clouds".to_string(),
            }],
        };
        
        let point = WeatherSource::response_to_datapoint("weather", &response);
        
        assert_eq!(point.source, "weather");
        assert_eq!(point.values.get("temperature"), Some(&22.5));
        assert_eq!(point.values.get("humidity"), Some(&65.0));
        assert_eq!(point.values.get("pressure"), Some(&1013.0));
        assert_eq!(point.values.get("wind_speed"), Some(&3.5));
        assert_eq!(point.values.get("clouds"), Some(&40.0));
        assert!(point.events.contains(&"Clouds".to_string()));
    }
    
    #[test]
    fn test_response_to_datapoint_minimal() {
        let response = WeatherResponse {
            main: MainData {
                temp: 20.0,
                humidity: 50.0,
                pressure: 1000.0,
                feels_like: 20.0,
            },
            wind: None,
            clouds: None,
            weather: vec![],
        };
        
        let point = WeatherSource::response_to_datapoint("weather", &response);
        
        assert_eq!(point.values.get("temperature"), Some(&20.0));
        assert!(!point.values.contains_key("wind_speed"));
        assert!(!point.values.contains_key("clouds"));
        assert!(point.events.is_empty());
    }
    
    #[test]
    fn test_parse_real_api_response() {
        // Test parsing an actual API response format
        let json = r#"{
            "coord": {"lon": -97.74, "lat": 30.27},
            "weather": [{"id": 801, "main": "Clouds", "description": "few clouds", "icon": "02d"}],
            "base": "stations",
            "main": {"temp": 22.5, "feels_like": 23.1, "temp_min": 20.0, "temp_max": 25.0, "pressure": 1013, "humidity": 65},
            "visibility": 10000,
            "wind": {"speed": 3.5, "deg": 180, "gust": 5.2},
            "clouds": {"all": 20},
            "dt": 1705500000,
            "sys": {"type": 2, "id": 2000, "country": "US", "sunrise": 1705490000, "sunset": 1705530000},
            "timezone": -21600,
            "id": 4671654,
            "name": "Austin",
            "cod": 200
        }"#;
        
        let response: WeatherResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.main.temp, 22.5);
        assert_eq!(response.main.humidity, 65.0);
        assert_eq!(response.wind.as_ref().unwrap().speed, 3.5);
        assert_eq!(response.clouds.as_ref().unwrap().all, 20.0);
        assert_eq!(response.weather[0].main, "Clouds");
    }
}
