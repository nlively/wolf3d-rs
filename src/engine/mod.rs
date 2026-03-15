/// Rendering engine — raycaster + sprite scaler.
///
/// Source counterparts:
///   WL_DRAW.C  — raycasting, wall drawing, floor/ceiling
///   WL_SCALE.C — dynamically scaled sprites
///   ID_VL.C    — VGA palette, pixel output
///   ID_VH.C    — higher-level drawing utilities
pub mod renderer;
pub mod scaler;
pub mod video;

pub use renderer::Renderer;
