/// Audio driver — corresponds to ID_SD.C.
///
/// Uses `rodio` for cross-platform PCM playback.
/// AdLib/OPL2 music emulation will require a separate OPL emulator crate
/// (e.g. `opl3` or `ymfm-rs`) or conversion to MIDI.
///
/// Priority queue matching the original's SD_PlaySound / SD_MusicOn logic.
use std::io::Cursor;

use anyhow::Result;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

pub struct AudioDriver {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    /// One sink per sound channel (original had 1 digitized + 1 music).
    sfx_sink: Sink,
    music_sink: Sink,
    /// Current SFX priority — new sounds only play if priority >= current.
    pub sfx_priority: u16,
}

impl AudioDriver {
    pub fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sfx_sink = Sink::try_new(&stream_handle)?;
        let music_sink = Sink::try_new(&stream_handle)?;
        Ok(Self {
            _stream,
            stream_handle,
            sfx_sink,
            music_sink,
            sfx_priority: 0,
        })
    }

    /// Play a raw PCM sound effect (8-bit unsigned, given sample rate).
    /// `priority` matches the original's priority system — lower priority
    /// sounds are ignored if a higher-priority sound is playing.
    pub fn play_sfx(&mut self, samples: Vec<u8>, sample_rate: u32, priority: u16) {
        if priority < self.sfx_priority && !self.sfx_sink.empty() {
            return;
        }
        self.sfx_priority = priority;
        self.sfx_sink.stop();

        // rodio expects signed 16-bit; convert u8 → i16
        let pcm: Vec<u8> = samples
            .iter()
            .flat_map(|&s| {
                let signed = (s as i16 - 128) * 256;
                signed.to_le_bytes()
            })
            .collect();

        // Build a minimal WAV in memory for Decoder
        let wav = build_wav(pcm, sample_rate, 1);
        if let Ok(source) = Decoder::new(Cursor::new(wav)) {
            self.sfx_sink.append(source);
            self.sfx_sink.play();
        }
    }

    pub fn stop_sfx(&mut self) {
        self.sfx_sink.stop();
        self.sfx_priority = 0;
    }

    pub fn set_music_volume(&self, vol: f32) {
        self.music_sink.set_volume(vol.clamp(0.0, 1.0));
    }

    pub fn set_sfx_volume(&self, vol: f32) {
        self.sfx_sink.set_volume(vol.clamp(0.0, 1.0));
    }
}

/// Build a minimal PCM WAV file in memory.
fn build_wav(pcm: Vec<u8>, sample_rate: u32, channels: u16) -> Vec<u8> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_len = pcm.len() as u32;
    let file_len = 36 + data_len;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&file_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());       // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes());        // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&pcm);
    wav
}
