//! A correctly declared derive compiles.
use clifacet::Facet;

#[derive(clifacet::clap::Parser, Facet)]
#[facet(input = Greeting)]
struct Cli {
    #[arg(long, alias = "nickname", short = 'n')]
    name: Option<String>,
}

/// The typed input the derive builds field by field.
struct Greeting {
    name: Option<String>,
}

fn main() {
    let _ = <Cli as Facet>::command();
}
