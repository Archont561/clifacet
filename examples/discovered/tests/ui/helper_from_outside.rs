//! A helper reached from outside its folder (CLI-50, AC#4).
//!
//! This file is compiled as its own crate that depends on the fixture, which is exactly
//! the position a sibling module — or another crate — is in: it is outside `src/commands/`.
//! A helper belongs to its folder, and `mod.rs` decides what leaves it, so this is `E0603`
//! at the line below rather than a path that happens to work.
use clifacet_discovered::commands::_shared::greeting;

fn main() {
    let _ = greeting();
}
