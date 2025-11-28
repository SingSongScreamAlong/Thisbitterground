# Contributing to This Bitter Ground

Thank you for your interest in contributing to This Bitter Ground!

## Project Structure

```
ThisBitterGround-RustECS/
├── sim/           # Rust ECS simulation (Bevy ECS)
├── gdext/         # GDExtension bridge (godot-rust/gdext)
├── godot/         # Godot 4 frontend (GDScript)
└── docs/          # Documentation
```

## Development Setup

### Prerequisites

- **Rust** (stable, 1.70+)
- **Godot 4.2+**
- **Cargo** (comes with Rust)

### Building

```bash
# Build the simulation library
cd sim
cargo build --release

# Build the GDExtension
cd gdext
cargo build --release

# Run tests
cd sim
cargo test --release
```

### Running in Godot

1. Open `godot/project.godot` in Godot 4
2. Press F5 to run the main scene

## Code Style

### Rust

- Follow standard Rust conventions (`cargo fmt`, `cargo clippy`)
- Document public APIs with `///` doc comments
- Write unit tests for new functionality
- Keep the FFI boundary stable (don't change `SQUAD_STRIDE` without versioning)

### GDScript

- Use static typing where possible
- Follow Godot's GDScript style guide
- Comment complex logic, especially buffer parsing

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes
3. Ensure all tests pass: `cargo test --release`
4. Push and create a PR
5. CodeRabbit will automatically review your PR
6. Address any feedback
7. Merge once approved

## Performance Guidelines

The simulation targets:
- **30 Hz** fixed timestep
- **3000+ units** at 60 FPS
- **< 16ms** per frame budget

When making changes to `sim/`:
- Profile before and after
- Avoid allocations in hot paths
- Use spatial partitioning for queries

## Questions?

Open an issue or start a discussion on GitHub.
