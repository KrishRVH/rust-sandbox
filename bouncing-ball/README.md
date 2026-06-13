# Bouncing Ball Physics Lab

An idiomatic Macroquad physics demo with material-specific balls, rotating polygon rails,
substepped collision solving, impact ripples, shockwaves, trails, and diagnostic overlays.

The simulation uses a fixed 120 Hz physics timestep and separates application orchestration,
simulation state, collision solving, rendering, and input controls into focused modules.

## Run

```bash
cargo run --release
```

Debug builds work, but release builds are the expected path for smooth animation.

## Controls

- `H` / `F1`: Toggle help and material legend.
- `Tab`: Toggle compact HUD.
- `P`: Pause or resume.
- `.` / `Right Arrow`: Step one fixed physics tick while paused.
- `R`: Reset with a new deterministic seed.
- `1`, `2`, `3`: Realtime, half speed, quarter speed.
- `V`: Toggle velocity vectors.
- `N`: Toggle collision normals.
- `T`: Toggle trails.
- `E`: Toggle visual effects.

Hover a ball to inspect its material, speed, mass, spin, and heat.

## Project Structure

```text
src/
  main.rs          # Macroquad window and frame loop only
  app.rs           # Fixed timestep, reset/pause/speed controls
  config.rs        # Tunable physics, arena, spawn, effect, render config
  geometry.rs      # Pure geometry helpers and tests
  input.rs         # View/diagnostic toggles

  sim/
    arena.rs       # Rotating polygon layer geometry
    ball.rs        # Physical ball state
    collision.rs   # Wall and ball contact solver
    effects.rs     # Ripple, shockwave, normal lifetimes
    material.rs    # Material properties and combination rules
    world.rs       # Simulation root and stats

  render/
    mod.rs         # Macroquad drawing, HUD, camera, visual diagnostics
```

## Physics Notes

- Ball-ball and ball-wall contacts are solved inside each physics substep.
- Rotating wall contact velocity is included in wall impulses.
- Restitution combines with the less-bouncy material, while friction uses a geometric mean.
- Positional correction is weighted by inverse mass.
- Spin uses a disk moment of inertia (`0.5 * m * r^2`).
- Heat is visual only and is rendered as glow/rim feedback, so material colors stay readable.
- A subtle visible vortex drive keeps the arena circulating instead of letting every ball settle
  into a pile.

The compact HUD displays fixed-step rate, substeps, frame collisions, total energy, and
energy drift from the initial state. Rotating walls, drag, and the vortex drive intentionally
exchange or dissipate energy, so drift is a diagnostic signal rather than a strict conservation
proof.

## Verification

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

For visual iteration on WSLg/Windows, run the app and capture the native window:

```bash
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/capture-window.ps1
```
