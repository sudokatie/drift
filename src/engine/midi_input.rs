//! MIDI input for Drift.
//!
//! Receives MIDI CC and note messages to control synthesis parameters in real-time.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Result};
use midir::{Ignore, MidiInput};

/// MIDI input event types.
#[derive(Debug, Clone, Copy)]
pub enum MidiInputEvent {
    /// Control change: controller (0-127), value (0-127)
    ControlChange(u8, u8),
    /// Note on: note (0-127), velocity (0-127)
    NoteOn(u8, u8),
    /// Note off: note (0-127), velocity (0-127)
    NoteOff(u8, u8),
}

/// Mapping from MIDI CC to parameter.
#[derive(Debug, Clone)]
pub struct CcMapping {
    /// CC controller number (0-127)
    pub cc_number: u8,
    /// Parameter name (e.g., "pitch", "filter", "volume")
    pub parameter: String,
    /// Output minimum value
    pub out_min: f64,
    /// Output maximum value
    pub out_max: f64,
}

impl CcMapping {
    /// Create a new CC mapping.
    pub fn new(cc_number: u8, parameter: &str, out_min: f64, out_max: f64) -> Self {
        Self {
            cc_number,
            parameter: parameter.to_string(),
            out_min,
            out_max,
        }
    }

    /// Map a CC value (0-127) to the output range.
    pub fn map_value(&self, cc_value: u8) -> f64 {
        let normalized = cc_value as f64 / 127.0;
        self.out_min + normalized * (self.out_max - self.out_min)
    }
}

/// Mapping from MIDI notes to triggers.
#[derive(Debug, Clone)]
pub struct NoteMapping {
    /// Note number (0-127), None = any note
    pub note: Option<u8>,
    /// Action to trigger
    pub action: NoteTriggerAction,
    /// Minimum velocity (0-127), notes below this are ignored
    pub min_velocity: u8,
}

/// Actions that can be triggered by MIDI notes.
#[derive(Debug, Clone)]
pub enum NoteTriggerAction {
    /// Trigger voice attack
    TriggerVoice(usize),
    /// Release voice
    ReleaseVoice(usize),
    /// Set pitch from note number
    SetPitch(usize),
}

impl NoteMapping {
    /// Create a mapping that triggers a voice on any note.
    pub fn trigger_any(voice_index: usize) -> Self {
        Self {
            note: None,
            action: NoteTriggerAction::TriggerVoice(voice_index),
            min_velocity: 1,
        }
    }

    /// Create a mapping that sets pitch from note number.
    pub fn pitch_from_note(voice_index: usize) -> Self {
        Self {
            note: None,
            action: NoteTriggerAction::SetPitch(voice_index),
            min_velocity: 1,
        }
    }

    /// Create a mapping for a specific note.
    pub fn specific_note(note: u8, action: NoteTriggerAction) -> Self {
        Self {
            note: Some(note),
            action,
            min_velocity: 1,
        }
    }
}

/// Configuration for MIDI input.
#[derive(Debug, Clone, Default)]
pub struct MidiInputConfig {
    /// MIDI channel to listen on (None = all channels)
    pub channel: Option<u8>,
    /// CC to parameter mappings
    pub cc_mappings: Vec<CcMapping>,
    /// Note trigger mappings
    pub note_mappings: Vec<NoteMapping>,
}

impl MidiInputConfig {
    /// Create a new empty configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a CC mapping.
    pub fn add_cc_mapping(&mut self, mapping: CcMapping) -> &mut Self {
        self.cc_mappings.push(mapping);
        self
    }

    /// Add a note mapping.
    pub fn add_note_mapping(&mut self, mapping: NoteMapping) -> &mut Self {
        self.note_mappings.push(mapping);
        self
    }

    /// Set channel filter.
    pub fn with_channel(&mut self, channel: u8) -> &mut Self {
        self.channel = Some(channel);
        self
    }

    /// Create a default mapping for common parameters.
    pub fn default_mappings() -> Self {
        let mut config = Self::new();
        
        // CC 1 (Mod Wheel) -> filter cutoff
        config.add_cc_mapping(CcMapping::new(1, "filter", 200.0, 8000.0));
        
        // CC 7 (Volume) -> volume
        config.add_cc_mapping(CcMapping::new(7, "volume", 0.0, 1.0));
        
        // CC 74 (Filter) -> filter cutoff (common on many controllers)
        config.add_cc_mapping(CcMapping::new(74, "filter", 200.0, 8000.0));
        
        // CC 71 (Resonance) -> filter resonance
        config.add_cc_mapping(CcMapping::new(71, "resonance", 0.5, 5.0));
        
        config
    }
}

/// Parameter update from MIDI input.
#[derive(Debug, Clone)]
pub struct ParameterUpdate {
    /// Parameter name
    pub parameter: String,
    /// New value
    pub value: f64,
}

/// Voice trigger from MIDI input.
#[derive(Debug, Clone)]
pub enum VoiceTrigger {
    /// Trigger attack
    Attack { voice_index: usize, velocity: f64 },
    /// Trigger release
    Release { voice_index: usize },
    /// Set pitch from MIDI note
    SetPitch { voice_index: usize, frequency: f64 },
}

/// MIDI input listener.
pub struct MidiInputListener {
    config: MidiInputConfig,
    param_receiver: Receiver<ParameterUpdate>,
    trigger_receiver: Receiver<VoiceTrigger>,
    stop_sender: Sender<()>,
    running: Arc<Mutex<bool>>,
}

impl MidiInputListener {
    /// Create a new MIDI input listener connected to the given port.
    pub fn new(port_name: Option<&str>, config: MidiInputConfig) -> Result<Self> {
        let midi_in = MidiInput::new("Drift MIDI Input")?;
        let ports = midi_in.ports();

        if ports.is_empty() {
            return Err(anyhow!("No MIDI input ports available"));
        }

        let port = if let Some(name) = port_name {
            ports
                .iter()
                .find(|p| {
                    midi_in
                        .port_name(p)
                        .map(|n| n.contains(name))
                        .unwrap_or(false)
                })
                .ok_or_else(|| anyhow!("MIDI port '{}' not found", name))?
                .clone()
        } else {
            ports[0].clone()
        };

        let port_name_actual = midi_in.port_name(&port)?;

        let (param_sender, param_receiver) = mpsc::channel::<ParameterUpdate>();
        let (trigger_sender, trigger_receiver) = mpsc::channel::<VoiceTrigger>();
        let (stop_sender, stop_receiver) = mpsc::channel::<()>();

        let running = Arc::new(Mutex::new(true));
        let running_clone = running.clone();
        let config_clone = config.clone();

        // Create MIDI input with callback
        let mut midi_in = MidiInput::new("Drift MIDI Input")?;
        midi_in.ignore(Ignore::None);

        let _conn = midi_in.connect(
            &port,
            "drift-input",
            move |_timestamp, message, _| {
                if message.len() < 2 {
                    return;
                }

                let status = message[0];
                let channel = status & 0x0F;
                let message_type = status & 0xF0;

                // Check channel filter
                if let Some(filter_channel) = config_clone.channel {
                    if channel != filter_channel {
                        return;
                    }
                }

                match message_type {
                    0xB0 => {
                        // Control Change
                        let cc = message[1];
                        let value = message[2];

                        for mapping in &config_clone.cc_mappings {
                            if mapping.cc_number == cc {
                                let mapped_value = mapping.map_value(value);
                                let _ = param_sender.send(ParameterUpdate {
                                    parameter: mapping.parameter.clone(),
                                    value: mapped_value,
                                });
                            }
                        }
                    }
                    0x90 => {
                        // Note On
                        let note = message[1];
                        let velocity = message[2];

                        if velocity == 0 {
                            // Note on with velocity 0 = note off
                            for mapping in &config_clone.note_mappings {
                                if mapping.note.is_none() || mapping.note == Some(note) {
                                    if let NoteTriggerAction::TriggerVoice(voice_index) =
                                        mapping.action
                                    {
                                        let _ = trigger_sender.send(VoiceTrigger::Release {
                                            voice_index,
                                        });
                                    }
                                }
                            }
                        } else if velocity >= 1 {
                            for mapping in &config_clone.note_mappings {
                                if velocity < mapping.min_velocity {
                                    continue;
                                }
                                if mapping.note.is_none() || mapping.note == Some(note) {
                                    match mapping.action {
                                        NoteTriggerAction::TriggerVoice(voice_index) => {
                                            let _ = trigger_sender.send(VoiceTrigger::Attack {
                                                voice_index,
                                                velocity: velocity as f64 / 127.0,
                                            });
                                        }
                                        NoteTriggerAction::ReleaseVoice(voice_index) => {
                                            let _ = trigger_sender.send(VoiceTrigger::Release {
                                                voice_index,
                                            });
                                        }
                                        NoteTriggerAction::SetPitch(voice_index) => {
                                            let frequency = midi_note_to_frequency(note);
                                            let _ = trigger_sender.send(VoiceTrigger::SetPitch {
                                                voice_index,
                                                frequency,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    0x80 => {
                        // Note Off
                        let note = message[1];

                        for mapping in &config_clone.note_mappings {
                            if mapping.note.is_none() || mapping.note == Some(note) {
                                if let NoteTriggerAction::TriggerVoice(voice_index) = mapping.action
                                {
                                    let _ =
                                        trigger_sender.send(VoiceTrigger::Release { voice_index });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            },
            (),
        )?;

        // Keep connection alive in a thread
        thread::spawn(move || {
            let _conn = _conn; // Move connection into thread
            loop {
                if stop_receiver.try_recv().is_ok() {
                    break;
                }
                if !*running_clone.lock().unwrap() {
                    break;
                }
                thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        eprintln!("MIDI input connected to: {}", port_name_actual);

        Ok(Self {
            config,
            param_receiver,
            trigger_receiver,
            stop_sender,
            running,
        })
    }

    /// Poll for parameter updates (non-blocking).
    pub fn poll_parameters(&self) -> Vec<ParameterUpdate> {
        let mut updates = Vec::new();
        while let Ok(update) = self.param_receiver.try_recv() {
            updates.push(update);
        }
        updates
    }

    /// Poll for voice triggers (non-blocking).
    pub fn poll_triggers(&self) -> Vec<VoiceTrigger> {
        let mut triggers = Vec::new();
        while let Ok(trigger) = self.trigger_receiver.try_recv() {
            triggers.push(trigger);
        }
        triggers
    }

    /// Get the current configuration.
    pub fn config(&self) -> &MidiInputConfig {
        &self.config
    }

    /// Stop the MIDI input listener.
    pub fn stop(&self) {
        *self.running.lock().unwrap() = false;
        let _ = self.stop_sender.send(());
    }
}

impl Drop for MidiInputListener {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Convert MIDI note number to frequency in Hz.
pub fn midi_note_to_frequency(note: u8) -> f64 {
    440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0)
}

/// List available MIDI input ports.
pub fn list_midi_input_ports() -> Result<Vec<String>> {
    let midi_in = MidiInput::new("Drift MIDI List")?;
    let ports = midi_in.ports();

    let names: Vec<String> = ports
        .iter()
        .filter_map(|p| midi_in.port_name(p).ok())
        .collect();

    Ok(names)
}

/// Get the default MIDI input port name.
pub fn default_input_port_name() -> Option<String> {
    let midi_in = MidiInput::new("Drift MIDI Default").ok()?;
    let ports = midi_in.ports();
    ports.first().and_then(|p| midi_in.port_name(p).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cc_mapping_value() {
        let mapping = CcMapping::new(1, "filter", 200.0, 8000.0);

        // CC 0 -> out_min
        assert!((mapping.map_value(0) - 200.0).abs() < 0.01);

        // CC 127 -> out_max
        assert!((mapping.map_value(127) - 8000.0).abs() < 0.01);

        // CC 64 (midpoint) -> middle of range
        let mid = mapping.map_value(64);
        assert!(mid > 4000.0 && mid < 4200.0);
    }

    #[test]
    fn test_cc_mapping_inverted_range() {
        // Inverted range (out_min > out_max)
        let mapping = CcMapping::new(1, "reverse", 1.0, 0.0);

        assert!((mapping.map_value(0) - 1.0).abs() < 0.01);
        assert!((mapping.map_value(127) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_midi_note_to_frequency() {
        // A4 = 440 Hz
        assert!((midi_note_to_frequency(69) - 440.0).abs() < 0.01);

        // A3 = 220 Hz
        assert!((midi_note_to_frequency(57) - 220.0).abs() < 0.01);

        // Middle C = ~261.63 Hz
        let c4 = midi_note_to_frequency(60);
        assert!(c4 > 261.0 && c4 < 262.0);
    }

    #[test]
    fn test_midi_input_config_builder() {
        let mut config = MidiInputConfig::new();
        config
            .add_cc_mapping(CcMapping::new(1, "filter", 200.0, 8000.0))
            .add_cc_mapping(CcMapping::new(7, "volume", 0.0, 1.0))
            .with_channel(0);

        assert_eq!(config.cc_mappings.len(), 2);
        assert_eq!(config.channel, Some(0));
    }

    #[test]
    fn test_default_mappings() {
        let config = MidiInputConfig::default_mappings();

        assert!(config.cc_mappings.len() >= 4);

        // Should have mod wheel mapping
        let mod_wheel = config.cc_mappings.iter().find(|m| m.cc_number == 1);
        assert!(mod_wheel.is_some());
    }

    #[test]
    fn test_note_mapping_trigger_any() {
        let mapping = NoteMapping::trigger_any(0);

        assert!(mapping.note.is_none());
        assert_eq!(mapping.min_velocity, 1);
    }

    #[test]
    fn test_note_mapping_specific() {
        let mapping = NoteMapping::specific_note(60, NoteTriggerAction::TriggerVoice(1));

        assert_eq!(mapping.note, Some(60));
    }

    #[test]
    fn test_list_midi_input_ports() {
        // Just verify it doesn't panic
        let result = list_midi_input_ports();
        assert!(result.is_ok());
    }
}
