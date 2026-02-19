//! Waveform widget for ratatui

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Widget},
};

/// A widget that displays audio waveform
pub struct Waveform<'a> {
    samples: &'a [f32],
    style: Style,
    block: Option<Block<'a>>,
}

impl<'a> Waveform<'a> {
    pub fn new(samples: &'a [f32]) -> Self {
        Self {
            samples,
            style: Style::default(),
            block: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Render the waveform in the given area
    fn render_waveform(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.samples.is_empty() {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;
        let center_y = area.y + (height / 2) as u16;

        // Downsample or upsample to fit width
        let samples_per_col = self.samples.len().max(1) as f32 / width as f32;

        for x in 0..width {
            // Get average sample value for this column
            let start_idx = (x as f32 * samples_per_col) as usize;
            let end_idx = ((x + 1) as f32 * samples_per_col) as usize;
            let end_idx = end_idx.min(self.samples.len());

            let avg = if start_idx < end_idx {
                let sum: f32 = self.samples[start_idx..end_idx].iter().sum();
                sum / (end_idx - start_idx) as f32
            } else if start_idx < self.samples.len() {
                self.samples[start_idx]
            } else {
                0.0
            };

            // Scale to height (-1 to 1 maps to full height)
            let half_height = (height / 2) as f32;
            let y_offset = (avg * half_height).clamp(-half_height, half_height) as i16;

            // Draw vertical line from center to sample position
            let screen_x = area.x + x as u16;

            if y_offset >= 0 {
                // Positive: draw from center upward
                for dy in 0..=y_offset.unsigned_abs() {
                    if center_y >= dy {
                        let y = center_y - dy;
                        if y >= area.y && y < area.y + area.height {
                            buf.set_string(screen_x, y, "│", self.style);
                        }
                    }
                }
            } else {
                // Negative: draw from center downward
                for dy in 0..=y_offset.unsigned_abs() {
                    let y = center_y + dy;
                    if y >= area.y && y < area.y + area.height {
                        buf.set_string(screen_x, y, "│", self.style);
                    }
                }
            }
        }

        // Draw center line
        for x in area.x..area.x + area.width {
            if center_y >= area.y && center_y < area.y + area.height {
                let cell = &buf[(x, center_y)];
                if cell.symbol() == " " {
                    buf.set_string(x, center_y, "─", Style::default());
                }
            }
        }
    }
}

impl Widget for Waveform<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner_area = match &self.block {
            Some(block) => {
                let inner = block.inner(area);
                block.clone().render(area, buf);
                inner
            }
            None => area,
        };

        self.render_waveform(inner_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_empty() {
        let waveform = Waveform::new(&[]);
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        waveform.render(area, &mut buf);
        // Should not panic
    }

    #[test]
    fn test_waveform_with_samples() {
        let samples = vec![0.0, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5, 0.0];
        let waveform = Waveform::new(&samples);
        let area = Rect::new(0, 0, 9, 5);
        let mut buf = Buffer::empty(area);
        waveform.render(area, &mut buf);
        // Should render without panic
    }

    #[test]
    fn test_waveform_with_block() {
        let samples = vec![0.5; 10];
        let waveform = Waveform::new(&samples)
            .block(ratatui::widgets::Block::default().title("Test"));
        let area = Rect::new(0, 0, 20, 10);
        let mut buf = Buffer::empty(area);
        waveform.render(area, &mut buf);
        // Should render without panic
    }

    #[test]
    fn test_waveform_style() {
        use ratatui::style::Color;
        let samples = vec![0.5; 5];
        let waveform = Waveform::new(&samples)
            .style(Style::default().fg(Color::Red));
        // Just testing that the method works
        assert_eq!(waveform.style.fg, Some(Color::Red));
    }
}
