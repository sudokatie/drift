//! Spectrum analyzer FFT engine and widget for ratatui
//!
//! Provides real-time frequency spectrum visualization using:
//! - Cooley-Tukey radix-2 FFT algorithm (pure Rust implementation)
//! - Logarithmic frequency scaling for perceptually accurate display
//! - Peak hold with exponential decay

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};
use std::f32::consts::PI;

/// Supported FFT sizes (must be powers of 2 for radix-2 FFT)
#[allow(dead_code)]
pub const FFT_SIZES: [usize; 4] = [256, 512, 1024, 2048];

/// Default FFT size
pub const DEFAULT_FFT_SIZE: usize = 1024;

/// Default number of display bars
pub const DEFAULT_BAR_COUNT: usize = 32;

/// Minimum number of display bars
pub const MIN_BAR_COUNT: usize = 8;

/// Maximum number of display bars
pub const MAX_BAR_COUNT: usize = 128;

/// Minimum dB floor (anything below this becomes -80dB)
pub const DB_FLOOR: f32 = -80.0;

/// Sample rate in Hz
pub const SAMPLE_RATE: f32 = 44100.0;

/// Peak decay rate per frame (0.95 = slow decay)
pub const PEAK_DECAY_RATE: f32 = 0.95;

/// Apply Hann window to input samples to reduce spectral leakage
///
/// The Hann window is defined as: w(n) = 0.5 * (1 - cos(2*PI*n / (N-1)))
pub fn hann_window(samples: &[f32]) -> Vec<f32> {
    let n = samples.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![0.0]; // Hann window is 0 at endpoints
    }

    samples
        .iter()
        .enumerate()
        .map(|(i, &sample)| {
            let window = 0.5 * (1.0 - (2.0 * PI * i as f32 / (n - 1) as f32).cos());
            sample * window
        })
        .collect()
}

/// Compute FFT using Cooley-Tukey radix-2 algorithm
///
/// Takes time-domain samples and FFT size, returns magnitude spectrum (positive frequencies only).
/// Input is zero-padded or truncated to match fft_size.
/// Returns fft_size/2 magnitude values (DC to Nyquist).
pub fn compute_fft(samples: &[f32], fft_size: usize) -> Vec<f32> {
    // Validate FFT size is power of 2
    assert!(
        fft_size.is_power_of_two(),
        "FFT size must be a power of 2"
    );

    // Prepare input: apply window, zero-pad or truncate
    let windowed = hann_window(samples);
    let mut real: Vec<f32> = vec![0.0; fft_size];
    let mut imag: Vec<f32> = vec![0.0; fft_size];

    // Copy windowed samples (truncate if needed)
    let copy_len = windowed.len().min(fft_size);
    real[..copy_len].copy_from_slice(&windowed[..copy_len]);

    // Bit-reversal permutation
    bit_reverse_permutation(&mut real, &mut imag);

    // Cooley-Tukey FFT
    cooley_tukey_fft(&mut real, &mut imag);

    // Compute magnitude spectrum (positive frequencies only: 0 to N/2)
    let half = fft_size / 2;
    let mut magnitudes = Vec::with_capacity(half);
    for i in 0..half {
        let mag = (real[i] * real[i] + imag[i] * imag[i]).sqrt();
        // Normalize by FFT size
        magnitudes.push(mag / fft_size as f32);
    }

    magnitudes
}

/// Bit-reversal permutation for in-place FFT
fn bit_reverse_permutation(real: &mut [f32], imag: &mut [f32]) {
    let n = real.len();
    let bits = n.trailing_zeros() as usize;

    for i in 0..n {
        let j = reverse_bits(i, bits);
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
    }
}

/// Reverse the lower `bits` bits of an integer
fn reverse_bits(mut x: usize, bits: usize) -> usize {
    let mut result = 0;
    for _ in 0..bits {
        result = (result << 1) | (x & 1);
        x >>= 1;
    }
    result
}

/// Cooley-Tukey radix-2 decimation-in-time FFT
fn cooley_tukey_fft(real: &mut [f32], imag: &mut [f32]) {
    let n = real.len();

    // Iterate through stages
    let mut size = 2;
    while size <= n {
        let half_size = size / 2;
        let angle_step = -2.0 * PI / size as f32;

        for start in (0..n).step_by(size) {
            for k in 0..half_size {
                let angle = angle_step * k as f32;
                let twiddle_real = angle.cos();
                let twiddle_imag = angle.sin();

                let even_idx = start + k;
                let odd_idx = start + k + half_size;

                // Butterfly operation
                let odd_real = real[odd_idx] * twiddle_real - imag[odd_idx] * twiddle_imag;
                let odd_imag = real[odd_idx] * twiddle_imag + imag[odd_idx] * twiddle_real;

                real[odd_idx] = real[even_idx] - odd_real;
                imag[odd_idx] = imag[even_idx] - odd_imag;
                real[even_idx] += odd_real;
                imag[even_idx] += odd_imag;
            }
        }

        size *= 2;
    }
}

/// Convert magnitude to dB scale with floor at DB_FLOOR
///
/// dB = 20 * log10(magnitude)
/// Values below DB_FLOOR are clamped to DB_FLOOR
pub fn magnitude_to_db(magnitude: f32) -> f32 {
    if magnitude <= 0.0 {
        return DB_FLOOR;
    }
    let db = 20.0 * magnitude.log10();
    db.max(DB_FLOOR)
}

/// Map FFT bins to display bars using logarithmic frequency scale
///
/// Low frequencies get more detail (more bins per bar) than high frequencies.
/// This matches human perception where we hear octaves logarithmically.
///
/// Returns a vector of bar magnitudes (in dB).
pub fn map_bins_to_bars(magnitudes: &[f32], num_bars: usize, sample_rate: f32) -> Vec<f32> {
    if magnitudes.is_empty() || num_bars == 0 {
        return Vec::new();
    }

    let num_bins = magnitudes.len();
    let fft_size = num_bins * 2;
    let bin_freq = sample_rate / fft_size as f32;

    // Define frequency range (skip DC, go up to Nyquist)
    let min_freq: f32 = 20.0; // 20 Hz (lowest audible)
    let max_freq: f32 = (sample_rate / 2.0).min(20000.0); // Nyquist or 20kHz

    let log_min = min_freq.ln();
    let log_max = max_freq.ln();

    let mut bars = Vec::with_capacity(num_bars);

    for bar_idx in 0..num_bars {
        // Logarithmic frequency boundaries for this bar
        let t0 = bar_idx as f32 / num_bars as f32;
        let t1 = (bar_idx + 1) as f32 / num_bars as f32;

        let freq_low = (log_min + t0 * (log_max - log_min)).exp();
        let freq_high = (log_min + t1 * (log_max - log_min)).exp();

        // Convert to bin indices
        let bin_low = ((freq_low / bin_freq).floor() as usize).max(1);
        let bin_high = ((freq_high / bin_freq).ceil() as usize).min(num_bins);

        // Average magnitude in this frequency range
        let mut sum = 0.0;
        let mut count = 0;
        for bin in bin_low..bin_high {
            if bin < magnitudes.len() {
                sum += magnitudes[bin];
                count += 1;
            }
        }

        let avg_mag = if count > 0 { sum / count as f32 } else { 0.0 };
        bars.push(magnitude_to_db(avg_mag));
    }

    bars
}

/// Represents a single spectrum bar with current magnitude and peak hold
#[derive(Debug, Clone)]
pub struct SpectrumBar {
    /// Current magnitude in dB
    pub magnitude: f32,
    /// Peak hold value in dB (decays over time)
    pub peak: f32,
    /// Center frequency of this bar in Hz
    pub frequency: f32,
}

impl SpectrumBar {
    pub fn new(frequency: f32) -> Self {
        Self {
            magnitude: DB_FLOOR,
            peak: DB_FLOOR,
            frequency,
        }
    }

    /// Update the bar with a new magnitude value
    pub fn update(&mut self, new_magnitude: f32) {
        self.magnitude = new_magnitude;
        // Update peak if new magnitude is higher
        if new_magnitude > self.peak {
            self.peak = new_magnitude;
        }
    }

    /// Apply peak decay (call once per frame)
    pub fn decay_peak(&mut self) {
        // Exponential decay toward DB_FLOOR
        // peak = peak * decay + floor * (1 - decay)
        self.peak = self.peak * PEAK_DECAY_RATE + DB_FLOOR * (1.0 - PEAK_DECAY_RATE);

        // Ensure peak doesn't go below current magnitude
        if self.peak < self.magnitude {
            self.peak = self.magnitude;
        }
    }
}

/// Calculate center frequencies for each bar using logarithmic scale
pub fn calculate_bar_frequencies(num_bars: usize, sample_rate: f32) -> Vec<f32> {
    let min_freq: f32 = 20.0;
    let max_freq: f32 = (sample_rate / 2.0).min(20000.0);
    let log_min = min_freq.ln();
    let log_max = max_freq.ln();

    (0..num_bars)
        .map(|i| {
            let t = (i as f32 + 0.5) / num_bars as f32;
            (log_min + t * (log_max - log_min)).exp()
        })
        .collect()
}

/// Widget that displays the frequency spectrum
pub struct SpectrumWidget<'a> {
    bars: &'a [SpectrumBar],
    block: Option<Block<'a>>,
}

impl<'a> SpectrumWidget<'a> {
    pub fn new(bars: &'a [SpectrumBar]) -> Self {
        Self { bars, block: None }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Get color for a bar based on its index (position in spectrum)
    fn bar_color(&self, bar_idx: usize, total_bars: usize) -> Color {
        if total_bars == 0 {
            return Color::Cyan;
        }

        let t = bar_idx as f32 / total_bars as f32;

        if t < 0.33 {
            // Low frequencies: cyan
            Color::Cyan
        } else if t < 0.66 {
            // Mid frequencies: green
            Color::Green
        } else {
            // High frequencies: magenta
            Color::Magenta
        }
    }

    /// Render the spectrum bars in the given area
    fn render_spectrum(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.bars.is_empty() {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let num_bars = self.bars.len();

        // Calculate bar width and spacing
        let bar_width = (width / num_bars).max(1);
        let total_bar_space = bar_width * num_bars;
        let start_x = area.x + ((width - total_bar_space.min(width)) / 2) as u16;

        // dB range for display
        let db_min = DB_FLOOR;
        let db_max = 0.0;
        let db_range = db_max - db_min;

        for (i, bar) in self.bars.iter().enumerate() {
            let bar_x = start_x + (i * bar_width) as u16;
            if bar_x >= area.x + area.width {
                break;
            }

            // Normalize magnitude to 0..1 range
            let normalized = ((bar.magnitude - db_min) / db_range).clamp(0.0, 1.0);
            let bar_height = (normalized * height as f32) as u16;

            // Normalize peak
            let peak_normalized = ((bar.peak - db_min) / db_range).clamp(0.0, 1.0);
            let peak_y = height as u16 - (peak_normalized * height as f32) as u16;

            let color = self.bar_color(i, num_bars);
            let style = Style::default().fg(color);
            let peak_style = Style::default().fg(Color::White);

            // Draw bar from bottom up
            for dy in 0..bar_height {
                let y = area.y + area.height - 1 - dy;
                if y >= area.y && y < area.y + area.height {
                    for dx in 0..bar_width.min((area.x + area.width - bar_x) as usize) {
                        let x = bar_x + dx as u16;
                        if x < area.x + area.width {
                            buf.set_string(x, y, "\u{2588}", style); // Full block
                        }
                    }
                }
            }

            // Draw peak marker
            let peak_screen_y = area.y + peak_y;
            if peak_screen_y >= area.y && peak_screen_y < area.y + area.height {
                for dx in 0..bar_width.min((area.x + area.width - bar_x) as usize) {
                    let x = bar_x + dx as u16;
                    if x < area.x + area.width {
                        buf.set_string(x, peak_screen_y, "\u{2594}", peak_style); // Upper one eighth block
                    }
                }
            }
        }
    }
}

impl Widget for SpectrumWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = match &self.block {
            Some(block) => {
                let inner = block.inner(area);
                block.clone().render(area, buf);
                inner
            }
            None => area,
        };

        self.render_spectrum(inner_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== FFT Engine Tests ==========

    #[test]
    fn test_hann_window_empty() {
        let result = hann_window(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_hann_window_single() {
        let result = hann_window(&[1.0]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], 0.0); // Window is 0 at endpoints
    }

    #[test]
    fn test_hann_window_endpoints() {
        let samples = vec![1.0; 10];
        let result = hann_window(&samples);
        assert_eq!(result.len(), 10);
        // Endpoints should be close to 0
        assert!(result[0].abs() < 0.001);
        assert!(result[9].abs() < 0.001);
    }

    #[test]
    fn test_hann_window_center_max() {
        let samples = vec![1.0; 9];
        let result = hann_window(&samples);
        // Center should be close to 1 (max window value)
        assert!((result[4] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_hann_window_symmetry() {
        let samples = vec![1.0; 8];
        let result = hann_window(&samples);
        // Window should be symmetric
        for i in 0..4 {
            assert!((result[i] - result[7 - i]).abs() < 0.001);
        }
    }

    #[test]
    fn test_reverse_bits() {
        assert_eq!(reverse_bits(0b000, 3), 0b000);
        assert_eq!(reverse_bits(0b001, 3), 0b100);
        assert_eq!(reverse_bits(0b010, 3), 0b010);
        assert_eq!(reverse_bits(0b011, 3), 0b110);
        assert_eq!(reverse_bits(0b100, 3), 0b001);
    }

    #[test]
    fn test_compute_fft_size() {
        let samples = vec![0.0; 1024];
        let result = compute_fft(&samples, 1024);
        assert_eq!(result.len(), 512); // Returns N/2 bins
    }

    #[test]
    fn test_compute_fft_zero_input() {
        let samples = vec![0.0; 256];
        let result = compute_fft(&samples, 256);
        // All magnitudes should be 0
        for mag in &result {
            assert!(mag.abs() < 0.001);
        }
    }

    #[test]
    fn test_compute_fft_dc_component() {
        // Constant signal should have energy only at DC
        let samples = vec![1.0; 256];
        let result = compute_fft(&samples, 256);
        // DC bin should have significant energy (though windowed)
        // Other bins should be relatively small
        let dc = result[0];
        let others_max = result[1..].iter().cloned().fold(0.0_f32, f32::max);
        assert!(dc > others_max * 0.1); // DC should be significant
    }

    #[test]
    fn test_compute_fft_sine_wave() {
        // Generate a 1kHz sine wave at 44100 sample rate
        let fft_size = 1024;
        let freq = 1000.0;
        let sample_rate = 44100.0;

        let samples: Vec<f32> = (0..fft_size)
            .map(|i| (2.0 * PI * freq * i as f32 / sample_rate).sin())
            .collect();

        let result = compute_fft(&samples, fft_size);

        // Find the bin with maximum energy
        let max_bin = result
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap()
            .0;

        // Expected bin: freq / (sample_rate / fft_size)
        let expected_bin = (freq / (sample_rate / fft_size as f32)).round() as usize;

        // Allow some tolerance due to windowing
        assert!((max_bin as i32 - expected_bin as i32).abs() <= 2);
    }

    #[test]
    fn test_compute_fft_zero_padding() {
        // Shorter input should be zero-padded
        let samples = vec![1.0; 100];
        let result = compute_fft(&samples, 256);
        assert_eq!(result.len(), 128);
    }

    #[test]
    fn test_compute_fft_truncation() {
        // Longer input should be truncated
        let samples = vec![1.0; 2000];
        let result = compute_fft(&samples, 1024);
        assert_eq!(result.len(), 512);
    }

    #[test]
    #[should_panic(expected = "FFT size must be a power of 2")]
    fn test_compute_fft_non_power_of_two() {
        let samples = vec![0.0; 100];
        compute_fft(&samples, 100); // Should panic
    }

    #[test]
    fn test_magnitude_to_db_zero() {
        assert_eq!(magnitude_to_db(0.0), DB_FLOOR);
    }

    #[test]
    fn test_magnitude_to_db_negative() {
        assert_eq!(magnitude_to_db(-1.0), DB_FLOOR);
    }

    #[test]
    fn test_magnitude_to_db_one() {
        // 20 * log10(1) = 0 dB
        assert!((magnitude_to_db(1.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_magnitude_to_db_half() {
        // 20 * log10(0.5) ~ -6.02 dB
        let db = magnitude_to_db(0.5);
        assert!((db - (-6.02)).abs() < 0.1);
    }

    #[test]
    fn test_magnitude_to_db_floor() {
        // Very small values should floor at DB_FLOOR
        let db = magnitude_to_db(0.0000001);
        assert_eq!(db, DB_FLOOR);
    }

    #[test]
    fn test_map_bins_to_bars_empty() {
        let result = map_bins_to_bars(&[], 32, SAMPLE_RATE);
        assert!(result.is_empty());
    }

    #[test]
    fn test_map_bins_to_bars_zero_bars() {
        let magnitudes = vec![0.1; 512];
        let result = map_bins_to_bars(&magnitudes, 0, SAMPLE_RATE);
        assert!(result.is_empty());
    }

    #[test]
    fn test_map_bins_to_bars_output_size() {
        let magnitudes = vec![0.1; 512];
        let result = map_bins_to_bars(&magnitudes, 32, SAMPLE_RATE);
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_map_bins_to_bars_all_zero() {
        let magnitudes = vec![0.0; 512];
        let result = map_bins_to_bars(&magnitudes, 32, SAMPLE_RATE);
        for bar in &result {
            assert_eq!(*bar, DB_FLOOR);
        }
    }

    #[test]
    fn test_map_bins_to_bars_logarithmic_coverage() {
        // With logarithmic scaling, bars should cover the frequency range
        let magnitudes = vec![0.1; 512];
        let bars = map_bins_to_bars(&magnitudes, 32, SAMPLE_RATE);
        let expected_db = magnitude_to_db(0.1);

        // Most bars should have valid data (some low freq bars may have no bins)
        let valid_bars: Vec<_> = bars.iter().filter(|&&db| db > DB_FLOOR).collect();
        assert!(valid_bars.len() >= 20, "Most bars should have data");

        // Valid bars should be close to expected
        for bar in valid_bars {
            assert!(
                (*bar - expected_db).abs() < 5.0,
                "Bar dB {} too far from expected {}",
                bar,
                expected_db
            );
        }
    }

    // ========== SpectrumBar Tests ==========

    #[test]
    fn test_spectrum_bar_new() {
        let bar = SpectrumBar::new(1000.0);
        assert_eq!(bar.frequency, 1000.0);
        assert_eq!(bar.magnitude, DB_FLOOR);
        assert_eq!(bar.peak, DB_FLOOR);
    }

    #[test]
    fn test_spectrum_bar_update() {
        let mut bar = SpectrumBar::new(1000.0);
        bar.update(-20.0);
        assert_eq!(bar.magnitude, -20.0);
        assert_eq!(bar.peak, -20.0);
    }

    #[test]
    fn test_spectrum_bar_peak_hold() {
        let mut bar = SpectrumBar::new(1000.0);
        bar.update(-10.0);
        bar.update(-30.0);
        assert_eq!(bar.magnitude, -30.0);
        assert_eq!(bar.peak, -10.0); // Peak should hold
    }

    #[test]
    fn test_spectrum_bar_peak_decay() {
        let mut bar = SpectrumBar::new(1000.0);
        bar.update(-10.0);
        bar.update(-80.0); // Drop magnitude
        bar.decay_peak();
        // Peak should have decayed but still be above magnitude
        assert!(bar.peak > bar.magnitude);
        assert!(bar.peak < -10.0);
    }

    #[test]
    fn test_spectrum_bar_peak_decay_floor() {
        let mut bar = SpectrumBar::new(1000.0);
        bar.update(-10.0);
        bar.update(DB_FLOOR);
        // Decay many times
        for _ in 0..1000 {
            bar.decay_peak();
        }
        // Peak should approach floor but not go below magnitude
        assert!(bar.peak >= bar.magnitude);
    }

    #[test]
    fn test_calculate_bar_frequencies() {
        let freqs = calculate_bar_frequencies(32, SAMPLE_RATE);
        assert_eq!(freqs.len(), 32);
        // Frequencies should be increasing
        for i in 1..freqs.len() {
            assert!(freqs[i] > freqs[i - 1]);
        }
        // First should be near 20Hz, last near 20kHz
        assert!(freqs[0] > 20.0);
        assert!(freqs[0] < 100.0);
        assert!(freqs[31] > 10000.0);
    }

    // ========== Widget Tests ==========

    #[test]
    fn test_spectrum_widget_empty() {
        let bars: Vec<SpectrumBar> = vec![];
        let widget = SpectrumWidget::new(&bars);
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        // Should not panic
    }

    #[test]
    fn test_spectrum_widget_with_bars() {
        let bars: Vec<SpectrumBar> = (0..32)
            .map(|i| {
                let mut bar = SpectrumBar::new(100.0 * (i + 1) as f32);
                bar.update(-40.0 + i as f32);
                bar
            })
            .collect();

        let widget = SpectrumWidget::new(&bars);
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        // Should render without panic
    }

    #[test]
    fn test_spectrum_widget_with_block() {
        let bars = vec![SpectrumBar::new(1000.0)];
        let widget = SpectrumWidget::new(&bars)
            .block(Block::bordered().title(" Spectrum "));
        let area = Rect::new(0, 0, 80, 20);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        // Should render without panic
    }

    #[test]
    fn test_spectrum_widget_small_area() {
        let bars: Vec<SpectrumBar> = (0..32).map(|i| SpectrumBar::new(100.0 * i as f32)).collect();
        let widget = SpectrumWidget::new(&bars);
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        // Should handle small areas without panic
    }

    #[test]
    fn test_spectrum_widget_zero_area() {
        let bars = vec![SpectrumBar::new(1000.0)];
        let widget = SpectrumWidget::new(&bars);
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        widget.render(area, &mut buf);
        // Should handle zero area without panic
    }

    #[test]
    fn test_spectrum_widget_bar_colors() {
        let bars: Vec<SpectrumBar> = (0..9).map(|i| SpectrumBar::new(100.0 * i as f32)).collect();
        let widget = SpectrumWidget::new(&bars);

        // Test color distribution
        assert_eq!(widget.bar_color(0, 9), Color::Cyan); // Low
        assert_eq!(widget.bar_color(4, 9), Color::Green); // Mid
        assert_eq!(widget.bar_color(8, 9), Color::Magenta); // High
    }

    #[test]
    fn test_spectrum_widget_bar_color_edge_cases() {
        let bars: Vec<SpectrumBar> = vec![];
        let widget = SpectrumWidget::new(&bars);

        // Zero total bars should return default
        assert_eq!(widget.bar_color(0, 0), Color::Cyan);
    }
}
