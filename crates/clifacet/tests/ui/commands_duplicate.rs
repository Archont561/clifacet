//! One module, one command: listing it twice is a compile error, not a doubled variant.

mod commands {
    clifacet::__commands!(greet, greet);
}

fn main() {}
