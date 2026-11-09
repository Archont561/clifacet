//! A helper is the folder's own business (CLI-50, AC#4).
//!
//! The compiler is what enforces it, so the enforcement is tested the way the rest of the
//! derive's diagnostics are: a file that must not compile, and the message it must produce.
//! Reaching a helper from outside its folder is `E0603` — private module — rather than a
//! path that quietly works because nothing declared it.
#[test]
fn a_helper_cannot_be_named_from_outside_its_folder() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/helper_from_outside.rs");
}

/// What a registry refuses, and the message a reader gets (CLI-58).
///
/// The attribute is written on a module, in the application's own file, so its mistakes are
/// as visible as any other declaration's. Each case here is a way of using it that looks
/// plausible and is not: on the wrong module, with a list of modules, and on a folder whose
/// header an `include!` cannot carry.
#[test]
fn a_registry_says_what_it_cannot_register() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/registry_*.rs");
}
