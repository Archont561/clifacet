#[derive(Default, Clone)]
struct AppContext;

#[clifacet::app]
struct BadContextSyntax {
    #[clifacet(context = "not_a_path")]
    cx: AppContext,
}

fn main() {}
