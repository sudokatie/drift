//! MIDI output for Drift.
//!
//! Maps data points to MIDI events and sends them to a MIDI port.

use std::sync::mpsc::{self, Sender};
use std::thread;

use anyhow::{anyhow, Result};
use midir::MidiOutput;

/// MIDI message types.
#[derive(Debug, Clone, Copy)]
pub enum MidiMessage {
    /// Note on: channel (0-15), note (0-127), velocity (0-127)
    NoteOn(u8, u8, u8),
    /// Note off: channel (0-15), note (0-127), velocity (0-127)
    NoteOff(u8, u8, u8),
    /// Control change: channel (0-15), controller (0-127), value (0-127)
    ControlChange(u8, u8, u8),
    /// Program change: channel (0-15), program (0-127)
    ProgramChange(u8, u8),
    /// Pitch bend: channel (0-15), value (0-16383, center at 8192)
    PitchBend(u8, u16),
}

impl MidiMessage {
    /// Convert to raw MIDI bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        match *self {
            MidiMessage::NoteOn(ch, note, vel) => vec![0x90 | (ch & 0x0F), note & 0x7F, vel & 0x7F],
            MidiMessage::NoteOff(ch, note, vel) => {
                vec![0x80 | (ch & 0x0F), note & 0x7F, vel & 0x7F]
            }
            MidiMessage::ControlChange(ch, ctrl, val) => {
                vec![0xB0 | (ch & 0x0F), ctrl & 0x7F, val & 0x7F]
            }
            MidiMessage::ProgramChange(ch, prog) => vec![0xC0 | (ch & 0x0F), prog & 0x7F],
            MidiMessage::PitchBend(ch, val) => {
                let lsb = (val & 0x7F) as u8;
                let msb = ((val >> 7) & 0x7F) as u8;
                vec![0xE0 | (ch & 0x0F), lsb, msb]
            }
        }
    }
}

/// Configuration for MIDI output.
#[derive(Debug, Clone)]
pub struct MidiConfig {
    /// MIDI channel (0-15)
    pub channel: u8,
    /// Base note for mapping (default: 60 = middle C)
    pub base_note: u8,
    /// Note range (how many semitones above base)
    pub note_range: u8,
    /// Velocity (0-127)
    pub velocity: u8,
    /// Use CC messages for continuous data
    pub use_cc: bool,
    /// CC controller number for continuous data
    pub cc_number: u8,
}

impl Default for MidiConfig {
    fn default() -> Self {
        Self {
            channel: 0,
            base_note: 48, // C3
            note_range: 36, // 3 octaves
            velocity: 100,
            use_cc: false,
            cc_number: 1, // Modulation wheel
        }
    }
}

/// MIDI output player.
pub struct MidiPlayer {
    sender: Sender<MidiPlayerCommand>,
    config: MidiConfig,
}

enum MidiPlayerCommand {
    Send(MidiMessage),
    Stop,
}

impl MidiPlayer {
    /// Create a new MIDI player connected to the given port.
    pub fn new(port_name: Option<&str>, config: MidiConfig) -> Result<Self> {
        let midi_out = MidiOutput::new("Drift MIDI Output")?;
        let ports = midi_out.ports();

        if ports.is_empty() {
            return Err(anyhow!("No MIDI output ports available"));
        }

        let port = if let Some(name) = port_name {
            ports
                .iter()
                .find(|p| {
                    midi_out
                        .port_name(p)
                        .map(|n| n.contains(name))
                        .unwrap_or(false)
                })
                .ok_or_else(|| anyhow!("MIDI port '{}' not found", name))?
                .clone()
        } else {
            ports[0].clone()
        };

        let port_name_actual = midi_out.port_name(&port)?;
        let conn = midi_out.connect(&port, "drift-output")?;

        let (sender, receiver) = mpsc::channel::<MidiPlayerCommand>();

        // Spawn thread to handle MIDI messages
        thread::spawn(move || {
            let mut conn = conn;
            while let Ok(cmd) = receiver.recv() {
                match cmd {
                    MidiPlayerCommand::Send(msg) => {
                        let bytes = msg.to_bytes();
                        let _ = conn.send(&bytes);
                    }
                    MidiPlayerCommand::Stop => break,
                }
            }
        });

        eprintln!("MIDI output connected to: {}", port_name_actual);

        Ok(Self { sender, config })
    }

    /// Map a normalized value (0.0-1.0) to a MIDI note and send note on.
    pub fn send_note(&self, value: f64) -> Result<()> {
        let value = value.clamp(0.0, 1.0);
        let note = self.config.base_note + (value * self.config.note_range as f64) as u8;
        let note = note.min(127);

        self.sender.send(MidiPlayerCommand::Send(MidiMessage::NoteOn(
            self.config.channel,
            note,
            self.config.velocity,
        )))?;

        Ok(())
    }

    /// Send note off for a given value.
    pub fn send_note_off(&self, value: f64) -> Result<()> {
        let value = value.clamp(0.0, 1.0);
        let note = self.config.base_note + (value * self.config.note_range as f64) as u8;
        let note = note.min(127);

        self.sender.send(MidiPlayerCommand::Send(MidiMessage::NoteOff(
            self.config.channel,
            note,
            0,
        )))?;

        Ok(())
    }

    /// Map a normalized value to CC and send.
    pub fn send_cc(&self, value: f64) -> Result<()> {
        let value = value.clamp(0.0, 1.0);
        let cc_value = (value * 127.0) as u8;

        self.sender
            .send(MidiPlayerCommand::Send(MidiMessage::ControlChange(
                self.config.channel,
                self.config.cc_number,
                cc_value,
            )))?;

        Ok(())
    }

    /// Send a raw MIDI message.
    pub fn send(&self, msg: MidiMessage) -> Result<()> {
        self.sender.send(MidiPlayerCommand::Send(msg))?;
        Ok(())
    }

    /// Stop the MIDI player.
    pub fn stop(&self) {
        let _ = self.sender.send(MidiPlayerCommand::Stop);
    }

    /// Get the current configuration.
    pub fn config(&self) -> &MidiConfig {
        &self.config
    }
}

impl Drop for MidiPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// List available MIDI output ports.
pub fn list_midi_ports() -> Result<Vec<String>> {
    let midi_out = MidiOutput::new("Drift MIDI List")?;
    let ports = midi_out.ports();

    let names: Vec<String> = ports
        .iter()
        .filter_map(|p| midi_out.port_name(p).ok())
        .collect();

    Ok(names)
}

/// Get the default MIDI output port name.
pub fn default_port_name() -> Option<String> {
    let midi_out = MidiOutput::new("Drift MIDI Default").ok()?;
    let ports = midi_out.ports();
    ports.first().and_then(|p| midi_out.port_name(p).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_message_note_on() {
        let msg = MidiMessage::NoteOn(0, 60, 100);
        assert_eq!(msg.to_bytes(), vec![0x90, 60, 100]);
    }

    #[test]
    fn test_midi_message_note_on_channel() {
        let msg = MidiMessage::NoteOn(5, 72, 80);
        assert_eq!(msg.to_bytes(), vec![0x95, 72, 80]);
    }

    #[test]
    fn test_midi_message_note_off() {
        let msg = MidiMessage::NoteOff(0, 60, 0);
        assert_eq!(msg.to_bytes(), vec![0x80, 60, 0]);
    }

    #[test]
    fn test_midi_message_cc() {
        let msg = MidiMessage::ControlChange(0, 1, 64);
        assert_eq!(msg.to_bytes(), vec![0xB0, 1, 64]);
    }

    #[test]
    fn test_midi_message_program_change() {
        let msg = MidiMessage::ProgramChange(0, 42);
        assert_eq!(msg.to_bytes(), vec![0xC0, 42]);
    }

    #[test]
    fn test_midi_message_pitch_bend_center() {
        let msg = MidiMessage::PitchBend(0, 8192);
        let bytes = msg.to_bytes();
        assert_eq!(bytes[0], 0xE0);
        // 8192 = 0x2000, LSB = 0x00, MSB = 0x40
        assert_eq!(bytes[1], 0x00);
        assert_eq!(bytes[2], 0x40);
    }

    #[test]
    fn test_midi_config_default() {
        let config = MidiConfig::default();
        assert_eq!(config.channel, 0);
        assert_eq!(config.base_note, 48);
        assert_eq!(config.note_range, 36);
        assert_eq!(config.velocity, 100);
    }

    #[test]
    fn test_list_midi_ports() {
        // Just verify it doesn't panic
        let result = list_midi_ports();
        assert!(result.is_ok());
    }
}
