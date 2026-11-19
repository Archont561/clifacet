//! A call with no modules is rejected and points to explicit modules or #[clifacet::app].

mod commands {
    clifacet::__commands!(name = "app");
}

fn main() {}
