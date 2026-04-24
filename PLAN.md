# Wolf3D → Rust Porting Plan

## Stack

| Layer | Crate | Notes |
|-------|-------|-------|
| Window / event loop | `winit 0.30` | Cross-platform, idiomatic Rust |
| Framebuffer | `pixels 0.15` | GPU-backed 320×200 pixel buffer via wgpu |
| Audio | `rodio 0.19` | PCM playback; OPL music needs emulator |
| Binary parsing | `byteorder 1.5` | Read original LE data files |
| Error handling | `anyhow` + `thiserror` | Standard Rust pattern |

## Source-to-module mapping

| Original file | Rust module | Status |
|---------------|-------------|--------|
| `ID_CA.C` | `src/assets/loader.rs` | Stub |
| `ID_MM.C`, `ID_PM.C` | (not needed — OS handles memory) | N/A |
| `ID_VL.C`, `ID_VH.C` | `src/engine/video.rs` | Stub |
| `ID_IN.C` | `src/input/handler.rs` | Basic key events wired |
| `ID_SD.C` | `src/audio/driver.rs` | PCM stub; AdLib TBD |
| `ID_US.C` | `src/ui/menu.rs` | Stub |
| `WL_DRAW.C` | `src/engine/renderer.rs` | DDA skeleton |
| `WL_SCALE.C` | `src/engine/scaler.rs` | Stub |
| `WL_PLAY.C`, `WL_GAME.C` | `src/game/state.rs` | Game loop wired |
| `WL_AGENT.C` | `src/game/player.rs` | Movement skeleton |
| `WL_ACT1.C`, `WL_ACT2.C` | `src/game/actor.rs` + `map.rs` | Spawn codes |
| `WL_STATE.C` | `src/game/ai.rs` | State machine skeleton |
| `WL_MENU.C` | `src/ui/menu.rs` | Stub |
| `WL_INTER.C` | `src/ui/intermission.rs` | Stub |

---

## Milestones

### Milestone 1 — Data pipeline ✦ (start here)
**Goal:** Load and inspect original game data without rendering anything.

- [x] **1a. Asset extraction tool** (`src/bin/extract.rs`)
  - Read `VGAHEAD.WL6` (chunk offsets) and `VGADICT.WL6` (Huffman tree)
  - Huffman-decode all chunks from `VGAGRAPH.WL6`
  - Dump PIC chunks as PNG for visual verification
  - Reference: `ID_CA.C::CAL_HuffExpand`, `CA_CacheGrChunk`

- [x] **1b. Map loader** (`src/assets/maps.rs`)
  - Read `MAPHEAD.WL6` (RLEW tag + 100 level offsets)
  - Read `GAMEMAPS.WL6`, Carmack-decompress each plane
  - RLEW-decompress to a flat `u16` tile array
  - Print a level's wall plane as ASCII art to verify
  - Reference: `ID_CA.C::CA_CacheMap`, `CA_RLEWexpand`

- [x] **1c. Sound loader** (`src/assets/sounds.rs`)
  - Read `AUDIOHED.WL6` (chunk offsets)
  - Read `AUDIOT.WL6` raw chunks
  - Identify and decode PC-speaker and digitized SFX chunks
  - Reference: `ID_CA.C::CA_LoadAllSounds`, `ID_SD.C`

---

### Milestone 2 — Renderer
**Goal:** Draw a traversable level using raycasted walls and a flat colour floor/ceiling.

- [x] **2a. Complete the DDA raycaster** (`src/engine/renderer.rs`)
  - Implement full DDA wall intersection (replace the placeholder in `cast_walls`)
  - Compute perpendicular distance correctly (avoid fish-eye)
  - Fill `depth_buf` for sprite clipping
  - Reference: `WL_DRAW.C::WallRefresh`, `HitVertWall` / `HitHorizWall`

- [x] **2b. Texture mapping**
  - Decode wall textures from graphics chunks into 64×64 RGBA slabs
  - Apply to wall columns (correct horizontal texture coordinate)
  - Shade E/W faces darker than N/S faces

- [ ] **2c. Door rendering**
  - Doors are a half-tile-offset wall column; interpolate `door.openness()`
  - Reference: `WL_DRAW.C::CastRay` door handling

- [ ] **2d. Sprite rendering** (`src/engine/scaler.rs`)
  - Sort visible actors by distance (back-to-front)
  - Project onto screen columns, depth-test against `depth_buf`
  - Scale 64×64 sprite to projected height

---

### Milestone 3 — Game loop
**Goal:** A playable level with enemies that can kill and be killed.

- [ ] **3a. Spawn level** (`src/game/map.rs`)
  - `GameMap::spawn_things` — complete all actor/item codes
  - Reference: `WL_ACT1.C::SpawnStatic`, `SpawnEnemy`, `SpawnPlayer`

- [ ] **3b. Player collision** (`src/game/player.rs`)
  - AABB collision vs walls and pushwalls (radius = 0.33 tiles)
  - Door interaction (press E → open nearest door)

- [ ] **3c. Enemy AI** (`src/game/ai.rs`)
  - Proper line-of-sight (ray vs wall grid, not just distance)
  - Patrol paths (follow direction tiles in plane 2)
  - Hitscan shooting (guard fires, player takes damage)
  - Reference: `WL_STATE.C::T_Stand`, `T_Path`, `T_Chase`, `T_Shoot`

- [ ] **3d. Weapons and combat**
  - Player attack: knife (melee), pistol/machinegun/chaingun (hitscan)
  - Damage falloff, hit detection, actor pain/death states
  - Reference: `WL_AGENT.C::GiveWeapon`, `T_Shoot`

- [ ] **3e. Items and pickups**
  - Food, medkits, ammo, keys, treasure
  - Reference: `WL_ACT1.C::PickupItem`

---

### Milestone 4 — Audio
**Goal:** Sound effects and music playing in-game.

- [ ] **4a. Digitized SFX** (`src/audio/driver.rs`)
  - Wire decoded SFX chunks to `AudioDriver::play_sfx`
  - Trigger sounds at the correct game events (shoot, door, pickup, death)
  - Reference: `ID_SD.C::SD_PlaySound`

- [ ] **4b. OPL2/AdLib music** (`src/audio/`)
  - Integrate an OPL2 emulator crate (e.g. `nuked-opl3` bindings, or a pure-Rust OPL)
  - Parse the IMF music format from the audio chunks
  - Feed OPL register writes to the emulator, mix output into rodio
  - Reference: `ID_SD.C::SDL_ALPlaySound`, `SDL_MusicPlayer`

---

### Milestone 5 — UI and flow
**Goal:** Full game loop from title screen to victory / game over.

- [ ] **5a. Main menu** (`src/ui/menu.rs`)
  - Render using game font bitmaps
  - Difficulty selection (I Am Death Incarnate → Can I Play Daddy)
  - Reference: `WL_MENU.C`

- [ ] **5b. HUD** (`src/ui/hud.rs`)
  - Score, health, ammo, lives, face sprite, key indicators
  - Reference: `WL_DRAW.C::DrawPlayBorder`, `StatusBarRefresh`

- [ ] **5c. Intermission** (`src/ui/intermission.rs`)
  - Level stats with animated counting
  - Reference: `WL_INTER.C::LevelCompleted`

- [ ] **5d. Save / load**
  - Port save game format or use `serde` for a friendlier format
  - Reference: `ID_US.C::USL_SaveGame`

---

### Milestone 6 — Polish
- [ ] Pushwalls (sliding secret walls)
- [ ] Elevator / level transitions between all 6 episodes
- [ ] Spear of Destiny support (conditional asset paths)
- [ ] Joystick / gamepad input via `gilrs`
- [ ] Mouse-look option
- [ ] Config file (key bindings, audio volumes)
- [ ] Wasm/web build target (pixels + winit both support wasm)

---

## Key reference points in the original source

| Topic | File | Key function |
|-------|------|-------------|
| DDA raycaster | `WL_DRAW.C` | `WallRefresh`, `CalcTics` |
| Sprite sorting | `WL_DRAW.C` | `DrawScaleds` |
| Door DDA | `WL_DRAW.C` | `CastRay` (door offset) |
| Actor spawn codes | `WL_ACT1.C` | `SpawnStatic`, `SpawnCheck` |
| Enemy state machine | `WL_STATE.C` | `statetype` array definitions |
| Huffman decode | `ID_CA.C` | `CAL_HuffExpand` |
| Carmack decompress | `ID_CA.C` | `CAL_CarmackExpand` |
| RLEW decompress | `ID_CA.C` | `CA_RLEWexpand` |
| AdLib music | `ID_SD.C` | `SDL_ALPlaySound` |

---

## Notes on fixed-point math

The original uses **16.16 fixed-point** (`long` = 32-bit).  Key constants:

```
TILEGLOBAL = 0x10000  (1 tile = Fixed::ONE)
FOCAL_LENGTH = 0x5700 (distance to projection plane)
MINDIST = 0x5800      (minimum wall distance, avoid div-by-zero)
```

All of these are already expressed as `Fixed` in `src/math/fixed.rs`.
The trig tables use `FINEANGLES = 3600` subdivisions.

## Getting the game data

You need the original Wolfenstein 3D shareware or full data files.
The shareware episode is legally free:

```
# Shareware data (Episode 1 only, .WL1 extension)
# Full game data uses .WL6 extension, Spear of Destiny uses .SOD

Place the following in assets/data/:
  VGAHEAD.WL6   VGADICT.WL6   VGAGRAPH.WL6
  MAPHEAD.WL6   GAMEMAPS.WL6
  AUDIOHED.WL6  AUDIOT.WL6
```
