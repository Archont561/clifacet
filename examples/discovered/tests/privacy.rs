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
