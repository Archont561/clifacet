//! What a folder publishes is what leaves it (CLI-50, AC#3).
//!
//! The other half — that the helper *module* cannot be named from out here — is the
//! compile-fail case in `tests/ui/helper_from_outside.rs`, driven by `tests/privacy.rs`.
//! This file is the positive side: `mod.rs` re-exported `greeting`, so `greeting` is a
//! public item of the folder even though the module holding it is not public.

#[test]
fn a_folder_publishes_exactly_what_its_mod_reexports() {
    assert_eq!(clifacet_discovered::commands::greeting(), "hello");
}
