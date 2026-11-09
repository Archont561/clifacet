// The commands of this application, discovered from this folder (CLI-50).
//
// Nothing here is registration: `#[clifacet::registry]` in `lib.rs` writes the `mod` lines,
// the tree, the `Request` and the `Cli` root, and reads the report `build.rs` wrote. What is
// left is the folder's own business — its helper, and what it re-exports.
//
// `list.rs` and `echo.rs` are commands, `db/` is a group, and `_shared.rs` is a helper: the
// folder says which, and nothing here repeats it.

// A helper: declared for this folder's commands by the registry, and never registered as one.
// The attribute writes its `mod` line, so this file only decides what it makes public —
// re-exported here so a test can reach it.
pub use _shared::greeting;
