//! A field declared as a `#[clifacet(..)]` argument parses and reports itself.

#[clifacet::command(input = Greeting)]
#[command(name = "greet", about = "Say hello")]
struct Cli {
    /// Who to greet. Defaults to `world`.
    #[clifacet(long, short = 'n', alias = "nickname", value_name = "NAME")]
    name: Option<String>,

    /// How many times to greet.
    #[clifacet(long, value_name = "COUNT")]
    repeat: u8,

    /// Passed to clap untouched, for anything the key table does not cover.
    #[arg(long, value_parser = clap::value_parser!(u8).range(1..=8))]
    level: u8,
}

/// The typed input the derive builds field by field.
struct Greeting {
    name: Option<String>,
    repeat: u8,
    level: u8,
}

fn main() {
    let cli = <Cli as clifacet::clap::Parser>::try_parse_from([
        "greet",
        "--nickname",
        "ada",
        "--repeat",
        "3",
        "--level",
        "2",
    ])
    .expect("the forwarded grammar parses");
    assert_eq!(cli.name.as_deref(), Some("ada"));
    assert_eq!(cli.repeat, 3);
    assert_eq!(cli.level, 2);

    // The generated help is clap's, built from the keys the attribute forwarded: the
    // short flag, the value name and the doc comment. `alias` is parsed but, as in clap,
    // not listed — `visible_alias` is the key that shows one, and both are forwarded.
    let help = <Cli as clap::CommandFactory>::command().render_help().to_string();
    assert!(help.contains("-n, --name <NAME>"), "short and value_name: {help}");
    assert!(help.contains("Who to greet"), "the doc comment is the help: {help}");
}
