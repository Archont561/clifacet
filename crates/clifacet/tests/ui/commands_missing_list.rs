//! A call with no modules and no build script is discovery that cannot run.

mod commands {
    clifacet::__commands!(name = "app");
}

fn main() {}
