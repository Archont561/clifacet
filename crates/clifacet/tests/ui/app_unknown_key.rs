#[clifacet::app(invalid_key = "value")]
struct BadApp {
    #[clifacet(long)]
    flag: bool,
}

fn main() {}
