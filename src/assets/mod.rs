/// Asset loading subsystem — corresponds to ID_CA.C (Cache Manager).
///
/// Wolf3D shipped with three main data archives:
///   VGAGRAPH / EGAGRAPH  — graphics chunks
///   GAMEMAPS             — map data (3-plane tile maps)
///   AUDIOT               — sound effects and music
///
/// Each archive is indexed by a header file produced by the asset compiler.
/// We read those files directly, matching the original chunk layout.
pub mod graphics;
pub mod loader;
pub mod maps;
pub mod sounds;
pub mod text;

pub use loader::AssetLoader;
