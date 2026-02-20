# lite-strtab

Crate for storing a lot of strings in a single buffer to save memory.

# Project Structure

- `lite-strtab/` - Main library crate
  - `src/` - Library source code
  - `benches/` - Benchmarks

# Code Guidelines

- Optimize for performance; use zero-cost abstractions, avoid allocations. Use arrays instead of maps if size is known ahead of time.
- Optimize for memory. Preallocate or trim if possible. Minimize memory use. Use smaller integers/types where appropriate. Use any other tricks that improve CPU or memory efficiency.
- Keep modules under 500 lines (excluding tests); split if larger.
- Place `use` inside functions only for `#[cfg]` conditional compilation.
- Prefer `core` over `std` where possible (`core::mem` over `std::mem`).

# Documentation Standards

- Document public items with `///`
- Add examples in docs where helpful
- Use `//!` for module-level docs
- Focus comments on "why" not "what"
- Use [`TypeName`] rustdoc links, not backticks.

# Verification

After code changes or for checks (testing/linting/building/docs/formatting), run `.cargo/verify.sh` (`.cargo/verify.ps1` on Windows). It echoes each command and runs the full suite, including core tests and any extra checks. Do this before returning to the user.
