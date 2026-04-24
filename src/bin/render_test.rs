/// Render a single frame to a PNG for visual inspection of the raycaster.
///
/// Run with:
///   cargo run --bin render_test
///
/// Output: output/render_test.png
use wolf3d_rs::assets::graphics::GraphicsCache;
use wolf3d_rs::assets::maps::{Level, MAP_SIZE, NUM_PLANES};
use wolf3d_rs::engine::renderer::{Renderer, View, VIEW_HEIGHT, VIEW_WIDTH};
use wolf3d_rs::math::Fixed;

fn make_test_level() -> Level {
    let mut planes = [[0u16; MAP_SIZE * MAP_SIZE]; NUM_PLANES];

    // 16x16 room in the top-left corner; border = tile 1 (solid wall).
    let room = 16usize;
    for y in 0..room {
        for x in 0..room {
            let is_border = x == 0 || y == 0 || x == room - 1 || y == room - 1;
            planes[0][y * MAP_SIZE + x] = if is_border { 1 } else { 0 };
        }
    }

    Level {
        name: "test".to_string(),
        planes,
        width: MAP_SIZE,
        height: MAP_SIZE,
        door_spawns: Vec::new(),
    }
}

fn main() {
    let level = make_test_level();

    let view = View {
        x: Fixed::from_f32(8.5),
        y: Fixed::from_f32(8.5),
        angle: 120,
    };

    let mut renderer = Renderer::new();

    let path = std::path::Path::new("assets");
    let graphics_cache = GraphicsCache::load(path).unwrap();

    let mut fb = vec![0u8; VIEW_WIDTH * VIEW_HEIGHT * 4];
    let textures: Vec<Vec<u8>> = graphics_cache.wall_textures;

    renderer.draw_frame(&mut fb, VIEW_WIDTH, &view, &level, &textures, &[]);

    std::fs::create_dir_all("output").expect("failed to create output/");
    image::save_buffer(
        "output/render_test.png",
        &fb,
        VIEW_WIDTH as u32,
        VIEW_HEIGHT as u32,
        image::ColorType::Rgba8,
    )
    .expect("failed to write PNG");

    println!("Wrote output/render_test.png ({VIEW_WIDTH}x{VIEW_HEIGHT})");
}
