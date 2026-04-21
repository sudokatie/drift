//! Terminal visualization for drift audio
//!
//! Provides a TUI interface showing:
//! - Waveform display
//! - Spectrum analyzer (real-time FFT)
//! - Current data values
//! - Playback controls

mod spectrum;
mod waveform;

pub use spectrum::{
    calculate_bar_frequencies, compute_fft, map_bins_to_bars, SpectrumBar, SpectrumWidget,
    DEFAULT_BAR_COUNT, DEFAULT_FFT_SIZE, MAX_BAR_COUNT, MIN_BAR_COUNT, PEAK_DECAY_RATE, SAMPLE_RATE,
};
pub use waveform::Waveform;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

use crate::engine::Engine;

/// Buffer for storing recent audio samples for visualization
pub struct SampleBuffer {
    samples: Vec<f32>,
    capacity: usize,
    write_pos: usize,
}

impl SampleBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: vec![0.0; capacity],
            capacity,
            write_pos: 0,
        }
    }

    /// Push a new sample into the buffer
    pub fn push(&mut self, sample: f32) {
        self.samples[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.capacity;
    }

    /// Get all samples in order (oldest to newest)
    pub fn get_samples(&self) -> Vec<f32> {
        let mut result = Vec::with_capacity(self.capacity);
        for i in 0..self.capacity {
            let idx = (self.write_pos + i) % self.capacity;
            result.push(self.samples[idx]);
        }
        result
    }

    /// Get the most recent N samples
    pub fn get_recent(&self, count: usize) -> Vec<f32> {
        let count = count.min(self.capacity);
        let samples = self.get_samples();
        samples[self.capacity - count..].to_vec()
    }
}

/// Visualization state
pub struct VizState {
    pub sample_buffer: Arc<Mutex<SampleBuffer>>,
    pub running: Arc<AtomicBool>,
    pub paused: bool,
    /// Whether spectrum analyzer is enabled
    pub spectrum_enabled: bool,
    /// Number of spectrum bars to display
    pub spectrum_bars: usize,
    /// Peak hold values for spectrum bars
    pub spectrum_peaks: Vec<SpectrumBar>,
}

impl VizState {
    pub fn new(buffer_size: usize) -> Self {
        let spectrum_bars = DEFAULT_BAR_COUNT;
        let frequencies = calculate_bar_frequencies(spectrum_bars, SAMPLE_RATE);
        let spectrum_peaks = frequencies
            .into_iter()
            .map(SpectrumBar::new)
            .collect();

        Self {
            sample_buffer: Arc::new(Mutex::new(SampleBuffer::new(buffer_size))),
            running: Arc::new(AtomicBool::new(true)),
            paused: false,
            spectrum_enabled: true,
            spectrum_bars,
            spectrum_peaks,
        }
    }

    /// Update the number of spectrum bars
    pub fn set_spectrum_bars(&mut self, count: usize) {
        let count = count.clamp(MIN_BAR_COUNT, MAX_BAR_COUNT);
        if count != self.spectrum_bars {
            self.spectrum_bars = count;
            let frequencies = calculate_bar_frequencies(count, SAMPLE_RATE);
            self.spectrum_peaks = frequencies.into_iter().map(SpectrumBar::new).collect();
        }
    }

    /// Update spectrum bars with new magnitude data
    pub fn update_spectrum(&mut self, magnitudes: &[f32]) {
        for (bar, &mag) in self.spectrum_peaks.iter_mut().zip(magnitudes.iter()) {
            bar.update(mag);
        }
    }

    /// Apply peak decay to all spectrum bars
    pub fn decay_spectrum_peaks(&mut self) {
        for bar in &mut self.spectrum_peaks {
            bar.decay_peak();
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

/// Run the visualization TUI
pub fn run_viz(
    _engine: Arc<Mutex<Engine>>,
    state: Arc<Mutex<VizState>>,
) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        {
            let state_guard = state.lock().unwrap();
            if !state_guard.is_running() {
                break;
            }
        }

        // Update spectrum data before drawing
        {
            let mut state_guard = state.lock().unwrap();
            if state_guard.spectrum_enabled && !state_guard.paused {
                // Get samples for FFT
                let buffer = state_guard.sample_buffer.lock().unwrap();
                let samples = buffer.get_recent(DEFAULT_FFT_SIZE);
                drop(buffer);

                // Compute FFT and map to bars
                let magnitudes = compute_fft(&samples, DEFAULT_FFT_SIZE);
                let bar_magnitudes =
                    map_bins_to_bars(&magnitudes, state_guard.spectrum_bars, SAMPLE_RATE);

                // Update spectrum bars
                state_guard.update_spectrum(&bar_magnitudes);
                state_guard.decay_spectrum_peaks();
            }
        }

        // Draw UI
        terminal.draw(|f| {
            let state_guard = state.lock().unwrap();
            draw_ui(f, &state_guard);
        })?;

        // Handle input
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                        state.lock().unwrap().stop();
                        break;
                    }
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        state.lock().unwrap().stop();
                        break;
                    }
                    (KeyCode::Char(' '), _) => {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.paused = !state_guard.paused;
                    }
                    (KeyCode::Char('s'), _) => {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.spectrum_enabled = !state_guard.spectrum_enabled;
                    }
                    (KeyCode::Char('+') | KeyCode::Char('='), _) => {
                        let mut state_guard = state.lock().unwrap();
                        let new_count = (state_guard.spectrum_bars + 8).min(MAX_BAR_COUNT);
                        state_guard.set_spectrum_bars(new_count);
                    }
                    (KeyCode::Char('-'), _) => {
                        let mut state_guard = state.lock().unwrap();
                        let new_count = state_guard.spectrum_bars.saturating_sub(8).max(MIN_BAR_COUNT);
                        state_guard.set_spectrum_bars(new_count);
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

fn draw_ui(f: &mut Frame, state: &VizState) {
    let area = f.area();

    if state.spectrum_enabled {
        // Layout: waveform on top, spectrum in middle, status at bottom
        // Recalculate to give status bar priority
        let main_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(3));
        let status_area = Rect::new(area.x, area.y + main_area.height, area.width, 3.min(area.height));

        let viz_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_area);

        // Draw waveform
        draw_waveform(f, viz_chunks[0], state);

        // Draw spectrum
        draw_spectrum(f, viz_chunks[1], state);

        // Draw status bar
        draw_status(f, status_area, state);
    } else {
        // Spectrum disabled: waveform takes full space
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),      // Waveform
                Constraint::Length(3),   // Status
            ])
            .split(area);

        // Draw waveform
        draw_waveform(f, chunks[0], state);

        // Draw status bar
        draw_status(f, chunks[1], state);
    }
}

fn draw_waveform(f: &mut Frame, area: Rect, state: &VizState) {
    let buffer = state.sample_buffer.lock().unwrap();
    let samples = buffer.get_recent(area.width as usize * 2);
    drop(buffer);

    let waveform = Waveform::new(&samples)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title(" Waveform "));

    f.render_widget(waveform, area);
}

fn draw_spectrum(f: &mut Frame, area: Rect, state: &VizState) {
    let widget = SpectrumWidget::new(&state.spectrum_peaks)
        .block(Block::default().borders(Borders::ALL).title(" Spectrum "));

    f.render_widget(widget, area);
}

fn draw_status(f: &mut Frame, area: Rect, state: &VizState) {
    let status = if state.paused { "PAUSED" } else { "PLAYING" };
    let status_color = if state.paused { Color::Yellow } else { Color::Green };

    let spectrum_status = if state.spectrum_enabled {
        format!("ON ({})", state.spectrum_bars)
    } else {
        "OFF".to_string()
    };
    let spectrum_color = if state.spectrum_enabled {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let text = Line::from(vec![
        Span::raw("  "),
        Span::styled(status, Style::default().fg(status_color)),
        Span::raw("  |  Spectrum: "),
        Span::styled(spectrum_status, Style::default().fg(spectrum_color)),
        Span::raw("  |  Space: pause  s: spectrum  +/-: bars  q: quit"),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_buffer_new() {
        let buffer = SampleBuffer::new(100);
        assert_eq!(buffer.capacity, 100);
        assert_eq!(buffer.samples.len(), 100);
    }

    #[test]
    fn test_sample_buffer_push() {
        let mut buffer = SampleBuffer::new(5);
        buffer.push(1.0);
        buffer.push(2.0);
        buffer.push(3.0);

        let samples = buffer.get_samples();
        // After pushing 3 values at indices 0,1,2, write_pos is at 3
        // get_samples reads from write_pos (3) and wraps
        // So order is: [idx 3, idx 4, idx 0, idx 1, idx 2] = [0, 0, 1, 2, 3]
        assert_eq!(samples[0], 0.0);  // idx 3 (empty)
        assert_eq!(samples[1], 0.0);  // idx 4 (empty)
        assert_eq!(samples[2], 1.0);  // idx 0
        assert_eq!(samples[3], 2.0);  // idx 1
        assert_eq!(samples[4], 3.0);  // idx 2
    }

    #[test]
    fn test_sample_buffer_wrap() {
        let mut buffer = SampleBuffer::new(3);
        buffer.push(1.0);
        buffer.push(2.0);
        buffer.push(3.0);
        buffer.push(4.0); // Wraps, overwrites first

        let samples = buffer.get_samples();
        assert_eq!(samples, vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_sample_buffer_get_recent() {
        let mut buffer = SampleBuffer::new(10);
        for i in 0..10 {
            buffer.push(i as f32);
        }

        let recent = buffer.get_recent(3);
        assert_eq!(recent, vec![7.0, 8.0, 9.0]);
    }

    #[test]
    fn test_viz_state_running() {
        let state = VizState::new(100);
        assert!(state.is_running());
        state.stop();
        assert!(!state.is_running());
    }

    #[test]
    fn test_viz_state_spectrum_defaults() {
        let state = VizState::new(100);
        assert!(state.spectrum_enabled);
        assert_eq!(state.spectrum_bars, DEFAULT_BAR_COUNT);
        assert_eq!(state.spectrum_peaks.len(), DEFAULT_BAR_COUNT);
    }

    #[test]
    fn test_viz_state_set_spectrum_bars() {
        let mut state = VizState::new(100);
        state.set_spectrum_bars(64);
        assert_eq!(state.spectrum_bars, 64);
        assert_eq!(state.spectrum_peaks.len(), 64);
    }

    #[test]
    fn test_viz_state_set_spectrum_bars_clamp_min() {
        let mut state = VizState::new(100);
        state.set_spectrum_bars(2);
        assert_eq!(state.spectrum_bars, MIN_BAR_COUNT);
    }

    #[test]
    fn test_viz_state_set_spectrum_bars_clamp_max() {
        let mut state = VizState::new(100);
        state.set_spectrum_bars(500);
        assert_eq!(state.spectrum_bars, MAX_BAR_COUNT);
    }

    #[test]
    fn test_viz_state_update_spectrum() {
        let mut state = VizState::new(100);
        let magnitudes = vec![-20.0; state.spectrum_bars];
        state.update_spectrum(&magnitudes);
        assert_eq!(state.spectrum_peaks[0].magnitude, -20.0);
    }

    #[test]
    fn test_viz_state_decay_spectrum_peaks() {
        let mut state = VizState::new(100);
        state.spectrum_peaks[0].update(-10.0);
        state.spectrum_peaks[0].update(-80.0);
        let peak_before = state.spectrum_peaks[0].peak;
        state.decay_spectrum_peaks();
        assert!(state.spectrum_peaks[0].peak < peak_before);
    }
}
