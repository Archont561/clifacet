#[derive(Default, Clone)]
struct AppContext;

#[clifacet::app(name = "test-valid", version = "1.0.0", about = "Valid app")]
struct ValidApp {
    #[clifacet(long)]
    flag: bool,
    #[clifacet(context)]
    cx: AppContext,
}

fn main() {}
