//! Price data source
//!
//! Fetches cryptocurrency and stock prices from CoinGecko API.

use super::{DataPoint, Source};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// CoinGecko API response for simple price
#[derive(Debug, Deserialize)]
struct CoinGeckoPrice {
    usd: f64,
    #[serde(default)]
    usd_24h_vol: Option<f64>,
    #[serde(default)]
    usd_24h_change: Option<f64>,
}

/// Configuration for price source
#[derive(Debug, Clone)]
pub struct PriceConfig {
    /// Symbols to track (e.g., ["bitcoin", "ethereum"])
    pub symbols: Vec<String>,
    /// Poll interval
    pub interval: Duration,
}

impl PriceConfig {
    /// Create config from settings map
    pub fn from_settings(settings: &HashMap<String, serde_yaml::Value>) -> Result<Self> {
        let symbols = settings
            .get("symbols")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(|| vec!["bitcoin".to_string()]);

        let interval_secs = settings
            .get("interval_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(60); // 1 minute default

        Ok(Self {
            symbols,
            interval: Duration::from_secs(interval_secs),
        })
    }
}

/// Tracks price history for volatility calculation
#[derive(Debug, Default)]
struct PriceHistory {
    prices: Vec<f64>,
    max_size: usize,
}

impl PriceHistory {
    fn new(max_size: usize) -> Self {
        Self {
            prices: Vec::with_capacity(max_size),
            max_size,
        }
    }

    fn push(&mut self, price: f64) {
        if self.prices.len() >= self.max_size {
            self.prices.remove(0);
        }
        self.prices.push(price);
    }

    /// Calculate volatility as standard deviation of returns
    fn volatility(&self) -> f64 {
        if self.prices.len() < 2 {
            return 0.0;
        }

        // Calculate returns
        let returns: Vec<f64> = self
            .prices
            .windows(2)
            .map(|w| (w[1] - w[0]) / w[0])
            .collect();

        // Mean of returns
        let mean: f64 = returns.iter().sum::<f64>() / returns.len() as f64;

        // Standard deviation
        let variance: f64 =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;

        variance.sqrt() * 100.0 // Convert to percentage
    }
}

/// Source that fetches cryptocurrency prices from CoinGecko
pub struct PriceSource {
    name: String,
    config: PriceConfig,
    running: Arc<AtomicBool>,
    sender: broadcast::Sender<DataPoint>,
    task: Option<JoinHandle<()>>,
}

impl PriceSource {
    /// Create a new price source
    pub fn new(name: impl Into<String>, config: PriceConfig) -> Self {
        let (sender, _) = broadcast::channel(16);
        Self {
            name: name.into(),
            config,
            running: Arc::new(AtomicBool::new(false)),
            sender,
            task: None,
        }
    }

    /// Build the API URL for CoinGecko
    fn build_url(symbols: &[String]) -> String {
        let ids = symbols.join(",");
        format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_vol=true&include_24hr_change=true",
            ids
        )
    }

    /// Fetch prices from API
    async fn fetch_prices(
        symbols: &[String],
    ) -> Result<HashMap<String, (f64, Option<f64>, Option<f64>)>> {
        let url = Self::build_url(symbols);
        let response = reqwest::get(&url)
            .await
            .context("failed to fetch price data")?;

        if !response.status().is_success() {
            bail!("price API returned status {}", response.status());
        }

        let data: HashMap<String, CoinGeckoPrice> = response
            .json()
            .await
            .context("failed to parse price response")?;

        let mut prices = HashMap::new();
        for (symbol, price_data) in data {
            prices.insert(
                symbol,
                (
                    price_data.usd,
                    price_data.usd_24h_vol,
                    price_data.usd_24h_change,
                ),
            );
        }

        Ok(prices)
    }

    /// Create DataPoint from price data
    fn prices_to_datapoint(
        name: &str,
        prices: &HashMap<String, (f64, Option<f64>, Option<f64>)>,
        histories: &HashMap<String, PriceHistory>,
    ) -> DataPoint {
        let mut point = DataPoint::new(name);

        // Add price data for each symbol
        for (symbol, (price, volume, change)) in prices {
            point = point.with_value(format!("{}_price", symbol), *price);

            if let Some(vol) = volume {
                point = point.with_value(format!("{}_volume", symbol), *vol);
            }

            if let Some(chg) = change {
                point = point.with_value(format!("{}_change_24h", symbol), *chg);

                // Add event for significant moves
                if chg.abs() > 5.0 {
                    let direction = if *chg > 0.0 { "pump" } else { "dump" };
                    point = point.with_event(format!("{}_{}", symbol, direction));
                }
            }

            // Add volatility if we have history
            if let Some(history) = histories.get(symbol) {
                let vol = history.volatility();
                point = point.with_value(format!("{}_volatility", symbol), vol);
            }
        }

        // Add aggregate values for first symbol (primary)
        if let Some((symbol, (price, _, change))) = prices.iter().next() {
            point = point.with_value("price", *price);
            if let Some(chg) = change {
                point = point.with_value("change_24h", *chg);
            }
            if let Some(history) = histories.get(symbol) {
                point = point.with_value("volatility", history.volatility());
            }
        }

        point
    }
}

impl Source for PriceSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        let name = self.name.clone();
        let symbols = self.config.symbols.clone();
        let interval = self.config.interval;
        let running = Arc::clone(&self.running);
        let sender = self.sender.clone();

        let task = tokio::spawn(async move {
            // Keep price history for volatility calculation
            let mut histories: HashMap<String, PriceHistory> = symbols
                .iter()
                .map(|s| (s.clone(), PriceHistory::new(60))) // 60 samples
                .collect();

            while running.load(Ordering::SeqCst) {
                match Self::fetch_prices(&symbols).await {
                    Ok(prices) => {
                        // Update histories
                        for (symbol, (price, _, _)) in &prices {
                            if let Some(history) = histories.get_mut(symbol) {
                                history.push(*price);
                            }
                        }

                        let point = Self::prices_to_datapoint(&name, &prices, &histories);
                        let _ = sender.send(point);
                    }
                    Err(e) => {
                        eprintln!("Price fetch error: {}", e);
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

impl Drop for PriceSource {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_config_from_settings() {
        let mut settings = HashMap::new();
        settings.insert(
            "symbols".to_string(),
            serde_yaml::Value::Sequence(vec![
                serde_yaml::Value::String("bitcoin".to_string()),
                serde_yaml::Value::String("ethereum".to_string()),
            ]),
        );
        settings.insert(
            "interval_secs".to_string(),
            serde_yaml::Value::Number(30.into()),
        );

        let config = PriceConfig::from_settings(&settings).unwrap();
        assert_eq!(config.symbols, vec!["bitcoin", "ethereum"]);
        assert_eq!(config.interval, Duration::from_secs(30));
    }

    #[test]
    fn test_price_config_defaults() {
        let settings = HashMap::new();
        let config = PriceConfig::from_settings(&settings).unwrap();

        assert_eq!(config.symbols, vec!["bitcoin"]);
        assert_eq!(config.interval, Duration::from_secs(60));
    }

    #[test]
    fn test_build_url() {
        let symbols = vec!["bitcoin".to_string(), "ethereum".to_string()];
        let url = PriceSource::build_url(&symbols);

        assert!(url.contains("bitcoin,ethereum"));
        assert!(url.contains("include_24hr_vol=true"));
        assert!(url.contains("include_24hr_change=true"));
    }

    #[test]
    fn test_price_history() {
        let mut history = PriceHistory::new(5);

        history.push(100.0);
        history.push(101.0);
        history.push(102.0);
        history.push(101.5);
        history.push(103.0);

        let vol = history.volatility();
        assert!(vol > 0.0);
        assert!(vol < 10.0); // Should be reasonable for small moves
    }

    #[test]
    fn test_price_history_max_size() {
        let mut history = PriceHistory::new(3);

        history.push(1.0);
        history.push(2.0);
        history.push(3.0);
        history.push(4.0);

        assert_eq!(history.prices.len(), 3);
        assert_eq!(history.prices, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_price_history_volatility_empty() {
        let history = PriceHistory::new(10);
        assert_eq!(history.volatility(), 0.0);
    }

    #[test]
    fn test_price_history_volatility_single() {
        let mut history = PriceHistory::new(10);
        history.push(100.0);
        assert_eq!(history.volatility(), 0.0);
    }

    #[test]
    fn test_prices_to_datapoint() {
        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), (50000.0, Some(1e10), Some(5.5)));

        let mut histories = HashMap::new();
        let mut btc_history = PriceHistory::new(10);
        btc_history.push(49000.0);
        btc_history.push(50000.0);
        histories.insert("bitcoin".to_string(), btc_history);

        let point = PriceSource::prices_to_datapoint("price", &prices, &histories);

        assert_eq!(point.source, "price");
        assert_eq!(point.values.get("bitcoin_price"), Some(&50000.0));
        assert_eq!(point.values.get("bitcoin_change_24h"), Some(&5.5));
        assert!(point.values.contains_key("bitcoin_volatility"));
        assert!(point.events.contains(&"bitcoin_pump".to_string()));
    }

    #[test]
    fn test_prices_to_datapoint_dump_event() {
        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), (50000.0, None, Some(-7.0)));

        let point = PriceSource::prices_to_datapoint("price", &prices, &HashMap::new());

        assert!(point.events.contains(&"bitcoin_dump".to_string()));
    }

    #[test]
    fn test_prices_to_datapoint_no_significant_move() {
        let mut prices = HashMap::new();
        prices.insert("bitcoin".to_string(), (50000.0, None, Some(2.0)));

        let point = PriceSource::prices_to_datapoint("price", &prices, &HashMap::new());

        assert!(!point.events.contains(&"bitcoin_pump".to_string()));
        assert!(!point.events.contains(&"bitcoin_dump".to_string()));
    }

    #[test]
    fn test_price_source_creation() {
        let config = PriceConfig {
            symbols: vec!["bitcoin".to_string()],
            interval: Duration::from_secs(60),
        };
        let source = PriceSource::new("test_price", config);

        assert_eq!(source.name(), "test_price");
        assert!(!source.is_running());
    }

    #[test]
    fn test_parse_coingecko_response() {
        let json = r#"{
            "bitcoin": {
                "usd": 50000.0,
                "usd_24h_vol": 25000000000.0,
                "usd_24h_change": 2.5
            },
            "ethereum": {
                "usd": 3000.0,
                "usd_24h_vol": 10000000000.0,
                "usd_24h_change": -1.2
            }
        }"#;

        let data: HashMap<String, CoinGeckoPrice> = serde_json::from_str(json).unwrap();

        assert_eq!(data.get("bitcoin").unwrap().usd, 50000.0);
        assert_eq!(data.get("bitcoin").unwrap().usd_24h_change, Some(2.5));
        assert_eq!(data.get("ethereum").unwrap().usd, 3000.0);
        assert_eq!(data.get("ethereum").unwrap().usd_24h_change, Some(-1.2));
    }
}
