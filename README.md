# Advanced Physics Simulation: Bouncing Balls in Rotating Dodecahedron

A sophisticated physics simulation featuring multiple bouncing balls with realistic material properties, ball-to-ball collisions, spin physics, air resistance, and visual effects inside rotating polygonal layers.

## New Physics Features

### 1. **Material System**
- **Rubber**: High bounce (85% restitution), light, high friction
- **Steel**: Heavy, moderate bounce (60% restitution), low friction  
- **Glass**: Very bouncy (95% restitution), smooth surface, medium weight

### 2. **Advanced Ball Physics**
- **Ball-to-ball collisions** with momentum conservation
- **Spin/Angular velocity** affecting bounce direction
- **Air resistance** using proper drag equation
- **Mass-based physics** (heavier balls fall differently)
- **Temperature system** (balls heat up from impacts)

### 3. **Visual Effects**
- **Material-colored ripples** based on colliding objects
- **Sound wave visualization** for hard impacts
- **Spin indicators** showing rotation
- **Temperature glow** (balls turn red when hot)
- **3D shading** with dynamic highlights

### 4. **Realistic Physics**
- **Variable ball sizes** (4-10 pixel radius)
- **Proper drag force**: F_drag = 0.5 * ρ * v² * C_d * A
- **Friction creates spin** on angled collisions
- **Energy tracking** for the whole system

## Project Structure

```
bouncing-ball-dodecahedron/
├── Cargo.toml              # Package configuration
├── src/
│   └── main.rs            # All simulation code
├── README.md              # This file
├── PHYSICS_REFERENCE.md   # Physics equations and concepts
└── TROUBLESHOOTING.md     # Common issues and solutions
```

## Code Architecture

The code is organized into several key components:

### 1. **Constants Section** (Lines 8-29)
- Physics parameters (gravity, friction, etc.)
- Visual effect settings
- Simulation configuration

### 2. **Data Structures**
- `CollisionInfo`: Stores collision event data
- `Ripple`: Animated water ripple effect
- `Ball`: Physics-enabled bouncing ball
- `PolygonLayer`: Rotating polygon with configurable properties
- `Simulation`: Main controller that manages everything

### 3. **Helper Functions**
- `rotate_point()`: 2D rotation math
- `circle_line_collision()`: Collision detection
- `reflect_velocity()`: Physics bounce calculation
- `hsl_to_rgb()`: Color generation

## How to Run

1. Make sure you have Rust installed (https://rustup.rs/)
2. Clone this repository
3. Run the simulation:
   ```bash
   cargo run --release
   ```

## Controls

- **SPACE**: Toggle debug information display
- **R**: Regenerate with new random configuration

## How the Advanced Physics Work

### Ball-to-Ball Collisions
The simulation uses **conservation of momentum** for realistic collisions:
```
Total momentum before = Total momentum after
m₁v₁ + m₂v₂ = m₁v₁' + m₂v₂'
```

### Spin Physics
- Friction at collision points converts linear motion to rotation
- Spin affects future bounces (like a pool ball with english)
- Visual indicator shows rotation speed and direction

### Material Properties
Each material has 6 properties:
1. **Density**: Affects mass and weight
2. **Restitution**: How much energy is retained in bounces
3. **Friction**: Surface roughness affecting spin
4. **Drag Coefficient**: Air resistance shape factor
5. **Thermal Conductivity**: How quickly heat spreads
6. **Color**: Visual identification

### Temperature System
- Collisions generate heat proportional to impact force
- Hot balls glow red and gradually cool down
- Temperature affects ball appearance but not physics (yet!)

## Extending the Code

### Easy Modifications

1. **Add new materials**:
   ```rust
   const MATERIAL_FOAM: Material = Material {
       density: 0.3,
       restitution: 0.7,
       friction: 0.9,
       drag_coefficient: 0.5,
       thermal_conductivity: 0.05,
       color_base: [0.9, 0.9, 0.3], // Yellow
   };
   ```

2. **Change physics constants**: 
   - Increase `SUBSTEPS` for more accurate collisions
   - Adjust `AIR_DENSITY` for different atmospheres
   - Modify `GRAVITY` for moon/space simulations

### Medium Complexity Extensions

1. **Magnetic fields**:
   - Add magnetic properties to steel balls
   - Create attraction/repulsion forces
   - Visualize field lines

2. **Breakable balls**:
   - Track damage accumulation
   - Split balls when damage exceeds threshold
   - Create particle effects on break

3. **Power-ups**:
   - Speed boosts
   - Size changes
   - Gravity reversal
   - Ghost mode (pass through walls)

### Advanced Extensions

1. **Fluid simulation**:
   - Fill bottom with water
   - Add buoyancy forces
   - Create splash effects

2. **Portals**:
   - Teleport balls between locations
   - Preserve momentum through portals
   - Add visual portal effects

3. **Gravity wells**:
   - Multiple gravity sources
   - Orbital mechanics
   - Tidal forces

4. **Destructible walls**:
   - Walls take damage from impacts
   - Break after threshold
   - Debris physics

## What Makes This Physics Sophisticated?

1. **Accurate Collision Detection**
   - 8 substeps per frame prevents tunneling
   - Circle-to-line segment math for precise wall collisions
   - Ball-to-ball uses proper sphere intersection

2. **Real Physics Equations**
   - Drag force: F = ½ρv²CₐA (actual aerodynamics)
   - Conservation of momentum in collisions
   - Angular momentum from friction
   - Proper mass calculations from density and volume

3. **Material Science**
   - Different materials behave differently
   - Combined material properties in collisions
   - Temperature effects from kinetic energy

4. **Multi-body Dynamics**
   - Every ball affects every other ball
   - System energy is tracked and conserved
   - Emergent behaviors from simple rules

5. **Visual Physics Feedback**
   - Ripple size shows impact force
   - Sound waves for loud collisions
   - Temperature glow indicates energy
   - Spin indicators show rotation

## Performance Considerations

- **Substeps**: 8 substeps ensures accuracy but costs performance
- **Ball count**: O(n²) complexity for ball-to-ball collisions
- **Effect limit**: Consider capping ripples/sound waves
- **Resolution**: Lower screen resolution = better performance

### Performance Tips:
1. Reduce `SUBSTEPS` to 4 for faster but less accurate physics
2. Limit balls to 6-8 for smooth performance on older hardware
3. Disable sound waves if performance is an issue
4. Reduce ripple segments from 60 to 30

## Learning Resources

- [Rust Book](https://doc.rust-lang.org/book/): Official Rust tutorial
- [Macroquad Examples](https://github.com/not-fl3/macroquad): Game engine examples
- [2D Physics](https://www.toptal.com/game/video-game-physics-part-i-an-introduction-to-rigid-body-dynamics): Understanding game physics
- [Real-Time Collision Detection](https://realtimecollisiondetection.net/): Advanced collision algorithms
- [Physics Simulations](https://www.myphysicslab.com/): Interactive physics demos

## Physics Concepts to Explore

1. **Conservation Laws**: Energy, momentum, angular momentum
2. **Collision Types**: Elastic, inelastic, perfectly inelastic
3. **Drag Forces**: Laminar vs turbulent flow
4. **Rotation Dynamics**: Moment of inertia, torque
5. **Material Science**: How real materials behave

## License

This is an educational project. Feel free to use and modify for learning purposes!

# Physics Reference Card

## Core Physics Equations Used in the Simulation

### 1. **Newton's Second Law**
```
F = ma
acceleration = Force / mass
```
Used for: Gravity, drag forces

### 2. **Kinematic Equations**
```
v = v₀ + at
x = x₀ + vt
```
Used for: Ball movement each frame

### 3. **Drag Force (Air Resistance)**
```
F_drag = ½ρv²CₐA
```
Where:
- ρ = air density
- v = velocity
- Cₐ = drag coefficient (shape factor)
- A = cross-sectional area

### 4. **Conservation of Momentum**
```
m₁v₁ + m₂v₂ = m₁v₁' + m₂v₂'
```
Used for: Ball-to-ball collisions

### 5. **Impulse-Momentum Theorem**
```
J = Δp = mΔv
Impulse = Force × time = change in momentum
```
Used for: Collision resolution

### 6. **Elastic Collision Formula**
```
v₁' = ((m₁-m₂)v₁ + 2m₂v₂)/(m₁+m₂)
v₂' = ((m₂-m₁)v₂ + 2m₁v₁)/(m₁+m₂)
```
Used for: Calculating post-collision velocities

### 7. **Coefficient of Restitution**
```
e = -(v₁' - v₂')/(v₁ - v₂)
```
Where e = 0 for perfectly inelastic, e = 1 for perfectly elastic

### 8. **Angular Momentum**
```
L = Iω = mr²ω
```
Where:
- I = moment of inertia
- ω = angular velocity
- For a disk: I = ½mr²

### 9. **Friction to Spin Conversion**
```
τ = rF_friction
α = τ/I
```
Where:
- τ = torque
- α = angular acceleration

### 10. **Energy Calculations**
```
KE_linear = ½mv²
KE_rotational = ½Iω²
Total Energy = KE_linear + KE_rotational + PE
```

## Material Properties Table

| Material | Density | Restitution | Friction | Drag Coef |
|----------|---------|-------------|----------|-----------|
| Rubber   | 1.0     | 0.85        | 0.8      | 0.47      |
| Steel    | 3.0     | 0.60        | 0.4      | 0.40      |
| Glass    | 2.0     | 0.95        | 0.2      | 0.45      |

## Key Physics Concepts

### Elastic vs Inelastic Collisions
- **Elastic**: Kinetic energy is conserved (bouncy ball)
- **Inelastic**: Some energy lost to heat/sound (real world)
- Our simulation uses partially elastic collisions

### Why Substeps?
Without substeps, fast-moving balls can "tunnel" through walls:
```
Frame 1: Ball is outside wall
Frame 2: Ball is inside wall (missed collision!)
```
Substeps check positions multiple times per frame.

### Frame Independence
Using `dt` (delta time) makes physics frame-rate independent:
```rust
position += velocity * dt;  // Works at any FPS
```

## Useful Constants
- g on Earth: 9.81 m/s²
- Air density at sea level: 1.225 kg/m³
- Drag coefficient sphere: 0.47
- Speed of sound: 343 m/s


# Troubleshooting Guide

## Common Issues and Solutions

### Build Errors

#### "error: linker 'cc' not found"
**Solution**: Install build tools for your OS:
- **Windows**: Install Visual Studio Build Tools
- **Linux**: `sudo apt install build-essential`
- **Mac**: `xcode-select --install`

#### "error[E0433]: failed to resolve: use of undeclared crate"
**Solution**: Run `cargo build` first to download dependencies

### Performance Issues

#### Low FPS / Stuttering
1. **Reduce ball count**: Edit `MIN_BALLS` and `MAX_BALLS` to lower values
2. **Reduce substeps**: Change `SUBSTEPS` from 8 to 4
3. **Disable effects**: Comment out sound wave creation
4. **Run in release mode**: Use `cargo run --release` (much faster!)

#### Balls Getting Stuck
- Increase `MIN_VELOCITY` constant
- Check that `RESTITUTION` isn't too low
- Ensure substeps aren't too low (minimum 4)

### Physics Bugs

#### Balls Passing Through Walls
- Increase `SUBSTEPS` (try 10 or 12)
- Reduce maximum ball velocity
- Check collision detection logic

#### Unrealistic Bounces
- Verify material properties make sense
- Check that normals are calculated correctly
- Ensure restitution is between 0 and 1

#### Energy Not Conserved
This is actually realistic! Real systems lose energy to:
- Heat (we simulate this)
- Sound (energy leaves the system)
- Deformation (not simulated)

### Visual Issues

#### Can't See Spin Indicators
- Make sure `angular_velocity` is high enough
- Check `SPIN_INDICATOR_LENGTH` constant
- Verify ball colors contrast with indicators

#### Ripples Not Showing
- Check that ripples are drawn before balls
- Verify opacity calculations
- Ensure collision detection is working

### Code Modifications

#### Adding a New Material
1. Define the material constant
2. Add to random selection in `Ball::new()`
3. Update the legend in `draw_info()`

#### Changing Physics
- Always use `dt` for time-based changes
- Remember units (pixels, seconds, radians)
- Test edge cases (zero mass, infinite velocity)

## Debug Tips

### Print Debugging
Add these anywhere to debug:
```rust
println!("Ball position: {:?}", ball.pos);
println!("Velocity: {:.2}", ball.vel.length());
```

### Visual Debugging
Draw debug info directly:
```rust
// Draw velocity vector
draw_line(
    ball.pos.x, ball.pos.y,
    ball.pos.x + ball.vel.x * 0.1, 
    ball.pos.y + ball.vel.y * 0.1,
    2.0, RED
);
```

### Common Rust Issues

#### "cannot borrow as mutable"
- Use `split_at_mut()` for multiple mutable references
- Clone data if needed (performance cost)
- Restructure to avoid simultaneous borrows

#### "does not live long enough"
- Check lifetime annotations
- Avoid storing references in structs
- Use owned data (`Vec<T>` not `&[T]`)

## Performance Profiling

To find bottlenecks:
1. Add timing code:
```rust
let start = macroquad::time::get_time();
// ... code to measure ...
let elapsed = macroquad::time::get_time() - start;
println!("Time: {:.3}ms", elapsed * 1000.0);
```

2. Common bottlenecks:
- Ball-to-ball collision (O(n²))
- Drawing many ripple segments
- Too many substeps

## Getting Help

1. **Rust errors**: Read them carefully - they're very helpful!
2. **Macroquad docs**: https://docs.rs/macroquad
3. **Rust book**: https://doc.rust-lang.org/book/
4. **Community**: r/rust, Rust Discord, Stack Overflow

Remember: If it compiles, it's probably memory-safe!