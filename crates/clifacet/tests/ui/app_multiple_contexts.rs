#[derive(Default, Clone)]
struct AppContext;

#[clifacet::app]
struct DoubleContext {
    #[clifacet(context)]
    cx1: AppContext,
    #[clifacet(context)]
    cx2: AppContext,
}

fn main() {}
