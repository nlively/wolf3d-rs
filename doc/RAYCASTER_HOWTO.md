# DDA Raycasting: How It Works

This document walks through the DDA (Digital Differential Analyzer) raycasting
algorithm used in `src/engine/renderer.rs`. It covers the logic that belongs in
the `cast_walls` TODO block, without being a code dump.

---

## The Mental Model

The renderer shoots one ray per screen column across a 60-degree field of view.
For each ray it finds the first wall tile in the level grid and records the
distance to it. `draw_walls` then uses those distances to project wall columns
at the correct height. The DDA is just the "find the first wall" step.

---

## Step 1: Decompose the ray into per-axis delta distances

The ray travels at angle `(cos, sin)`. For each unit step in X, the ray also
travels some fixed Y distance, and vice versa. These **delta distances** are:

```
delta_dist_x = |1 / cos|
delta_dist_y = |1 / sin|
```

They represent how far you travel along the ray to cross one full tile boundary
in each axis. Compute them once up front; they stay constant for the whole march.

`Fixed::abs()` and the trig tables in `self.trig` have everything you need.

---

## Step 2: Compute the first boundary crossing and step direction

The player sits at some fractional position inside a tile — not at a corner. So
the *first* X-boundary and *first* Y-boundary are at different distances than
subsequent ones.

- **Step direction** (`step_x`, `step_y`): +1 or -1 based on the sign of cos/sin.
- **Initial side distances**: use the fractional part of the player's position
  (`Fixed::frac()`) to measure how far you already are into the current tile.
  - If stepping in the positive direction, the first crossing is `(1.0 - frac) * delta`.
  - If stepping in the negative direction, it's `frac * delta`.

---

## Step 3: March the ray — always advance the closer boundary

At each iteration you have two candidates:

- `side_dist_x`: distance along the ray to the next X-grid crossing
- `side_dist_y`: distance along the ray to the next Y-grid crossing

**Always advance whichever is smaller.** When you advance X:

```
map_x += step_x
side_dist_x += delta_dist_x
```

And symmetrically for Y. Track which axis you just crossed — you'll need it for
face detection and texture coordinates.

This is the entire DDA insight: no square roots, no pixel-by-pixel tracing, just
two accumulators racing each other across the tile grid.

---

## Step 4: Check for a wall hit

After each step, look up the tile at `(map_x, map_y)` in the level. If it's
non-zero, you've hit a wall. At that point record:

- **Which axis you crossed last** — an X-crossing is an E/W wall face
  (`ew_face = true`); a Y-crossing is a N/S face.
- **The distance** (see Step 5 — don't use the raw value yet).
- **The hit tile number** for `ColumnHit::texture`.

---

## Step 5: Fix the fisheye distortion

Raw ray length causes a fisheye warp: rays at the edges of the FOV travel
farther than center rays to reach the same wall, making the image bow outward.

Wolf3D corrects this with the **perpendicular distance** — the distance from
the player to the wall projected onto the camera plane, not the actual ray
length. In practice this means using the side-distance accumulator for the axis
that was *not* the last one crossed (it already holds the right value without
any trig), or equivalently multiplying the ray length by the cosine of the
column's angular offset from center.

Store this corrected value in `ColumnHit::dist`. `draw_walls` uses it directly
at the wall-height projection formula.

---

## Step 6: Compute the texture X coordinate

Once you know which face was hit, the horizontal texture coordinate is the
fractional position along that tile edge, scaled to `0..63`:

- **E/W face** (X-crossing): the fraction comes from the Y component of the
  exact hit point on the wall.
- **N/S face** (Y-crossing): the fraction comes from the X component.

The exact hit point can be recovered from the player position plus the
perpendicular distance times the ray direction for the relevant axis.

---

## Summary

| Step | What happens |
|------|-------------|
| 1 | Compute `delta_dist_x` and `delta_dist_y` from `|1/cos|` and `|1/sin|` |
| 2 | Compute initial side distances from player fractional position; set step signs |
| 3 | Loop: advance the smaller accumulator, step `map_x` or `map_y` |
| 4 | After each step, check if the tile is a wall |
| 5 | On hit, correct for fisheye using the perpendicular distance |
| 6 | Derive the texture X coordinate from the fractional hit position |

The outer loop in `cast_walls` and the projection in `draw_walls` are already
wired up — the DDA fills the gap between them.
