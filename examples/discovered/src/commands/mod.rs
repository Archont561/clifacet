//! The commands of this application, discovered from this folder (CLI-50).
//!
//! The call has no list: `build.rs` scanned the folder and reported what is in it, and this
//! call reads that report. `list.rs` and `echo.rs` are commands, `db/` is a group, and
//! `_shared.rs` is a helper — the folder says which, and nothing here repeats it.

// A helper: declared for this folder's commands, and never registered as one. Re-exported
// so a test can reach it, which is also how `mod.rs` decides what a helper makes public.
mod _shared;

pub use _shared::greeting;

clifacet::commands!(
    output = crate::Said,
    name = "discovered",
    about = "A proof that a folder registers itself"
);
