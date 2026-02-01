//! WAV file recorder
//!
//! Records audio output to WAV files.

use anyhow::{Context, Result};
use hound::{SampleFormat, WavSpec, WavWriter};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// WAV file recorder
pub struct Recorder {
    writer: WavWriter<BufWriter<File>>,
    sample_rate: u32,
    samples_written: u64,
}

impl Recorder {
    /// Create a new recorder
    ///
    /// # Arguments
    /// * `path` - Output file path
    /// * `sample_rate` - Sample rate in Hz
    pub fn new(path: &Path, sample_rate: u32) -> Result<Self> {
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };

        let writer = WavWriter::create(path, spec)
            .with_context(|| format!("failed to create WAV file: {:?}", path))?;

        Ok(Self {
            writer,
            sample_rate,
            samples_written: 0,
        })
    }

    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of samples written
    pub fn samples_written(&self) -> u64 {
        self.samples_written
    }

    /// Get the duration recorded in seconds
    pub fn duration_secs(&self) -> f64 {
        self.samples_written as f64 / self.sample_rate as f64
    }

    /// Write a single sample
    pub fn write_sample(&mut self, sample: f32) -> Result<()> {
        self.writer
            .write_sample(sample)
            .context("failed to write sample")?;
        self.samples_written += 1;
        Ok(())
    }

    /// Write a buffer of samples
    pub fn write_buffer(&mut self, buffer: &[f32]) -> Result<()> {
        for &sample in buffer {
            self.writer
                .write_sample(sample)
                .context("failed to write sample")?;
        }
        self.samples_written += buffer.len() as u64;
        Ok(())
    }

    /// Finalize the WAV file
    ///
    /// This must be called to properly close the file and write the header.
    pub fn finalize(self) -> Result<()> {
        self.writer.finalize().context("failed to finalize WAV file")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_recorder_creation() {
        let file = NamedTempFile::new().unwrap();
        let recorder = Recorder::new(file.path(), 44100).unwrap();

        assert_eq!(recorder.sample_rate(), 44100);
        assert_eq!(recorder.samples_written(), 0);
        assert_eq!(recorder.duration_secs(), 0.0);
    }

    #[test]
    fn test_recorder_write_sample() {
        let file = NamedTempFile::new().unwrap();
        let mut recorder = Recorder::new(file.path(), 44100).unwrap();

        recorder.write_sample(0.5).unwrap();
        recorder.write_sample(-0.5).unwrap();

        assert_eq!(recorder.samples_written(), 2);
    }

    #[test]
    fn test_recorder_write_buffer() {
        let file = NamedTempFile::new().unwrap();
        let mut recorder = Recorder::new(file.path(), 44100).unwrap();

        let buffer = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        recorder.write_buffer(&buffer).unwrap();

        assert_eq!(recorder.samples_written(), 5);
    }

    #[test]
    fn test_recorder_duration() {
        let file = NamedTempFile::new().unwrap();
        let mut recorder = Recorder::new(file.path(), 44100).unwrap();

        // Write 1 second of samples
        for _ in 0..44100 {
            recorder.write_sample(0.0).unwrap();
        }

        assert!((recorder.duration_secs() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_recorder_finalize() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        let mut recorder = Recorder::new(&path, 44100).unwrap();
        recorder.write_sample(0.5).unwrap();
        recorder.finalize().unwrap();

        // Verify file exists and has content
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_recorder_produces_valid_wav() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path().to_path_buf();

        // Write some samples
        {
            let mut recorder = Recorder::new(&path, 44100).unwrap();
            for i in 0..1000 {
                let sample = (i as f32 / 1000.0 * std::f32::consts::PI * 2.0).sin();
                recorder.write_sample(sample).unwrap();
            }
            recorder.finalize().unwrap();
        }

        // Read back and verify
        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();

        assert_eq!(spec.channels, 1);
        assert_eq!(spec.sample_rate, 44100);
        assert_eq!(spec.bits_per_sample, 32);
        assert_eq!(spec.sample_format, SampleFormat::Float);

        let samples: Vec<f32> = reader.into_samples().map(|s| s.unwrap()).collect();
        assert_eq!(samples.len(), 1000);
    }
}
