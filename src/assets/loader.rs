/// Top-level asset loader — wraps graphics, map, and audio loading.
///
/// Source counterpart: ID_CA.C  (CA_Startup / CA_LoadAllSounds / CA_CacheMap)
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::assets::graphics::GraphicsCache;
use crate::assets::maps::MapCache;
use crate::assets::sounds::SoundCache;

pub struct AssetLoader {
    pub base_path: PathBuf,
    pub graphics: GraphicsCache,
    pub maps: MapCache,
    pub sounds: SoundCache,
}

impl AssetLoader {
    /// `base_path` should point to the directory containing the original
    /// Wolf3D data files (VGAGRAPH.WL6, GAMEMAPS.WL6, AUDIOT.WL6, etc.).
    pub fn load(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();

        let graphics = GraphicsCache::load(&base_path)
            .context("loading graphics")?;
        let maps = MapCache::load(&base_path)
            .context("loading maps")?;
        let sounds = SoundCache::load(&base_path)
            .context("loading sounds")?;

        Ok(Self { base_path, graphics, maps, sounds })
    }
}
