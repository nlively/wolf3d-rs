/// Sound asset loader — corresponds to CA_LoadAllSounds in ID_CA.C.
///
/// AUDIOT.WL6 contains digitized samples and AdLib instrument data,
/// indexed by AUDIOHED.WL6.
use std::path::Path;

use anyhow::Result;

/// A single decoded sound effect (PCM 8-bit unsigned, 7000 Hz like the
/// original digitized sounds).
pub struct SoundEffect {
    pub id: usize,
    pub samples: Vec<u8>,
    pub sample_rate: u32,
}

/// An AdLib music track (raw OPL2 register write sequence).
pub struct MusicTrack {
    pub id: usize,
    pub data: Vec<u8>,
}

pub struct SoundCache {
    pub sfx: Vec<Option<SoundEffect>>,
    pub music: Vec<Option<MusicTrack>>,
}

impl SoundCache {
    pub fn load(base: &Path) -> Result<Self> {
        // TODO:
        //   1. Read AUDIOHED.WL6 for chunk offsets and lengths.
        //   2. Read AUDIOT.WL6 raw bytes.
        //   3. Decode each chunk by type (DigiSound vs AdLib Instrument vs Music).
        log::warn!("SoundCache::load — stub, no audio loaded from {:?}", base);
        Ok(Self { sfx: Vec::new(), music: Vec::new() })
    }

    pub fn sfx(&self, id: usize) -> Option<&SoundEffect> {
        self.sfx.get(id)?.as_ref()
    }

    pub fn music(&self, id: usize) -> Option<&MusicTrack> {
        self.music.get(id)?.as_ref()
    }
}
