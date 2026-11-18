//! Tests for Figment-backed layered configuration and command mapping (CLI-59, decision-22).

#![cfg(feature = "settings-figment")]

use std::path::{Path, PathBuf};

use clap::{Arg, Command};
use clifacet::policy::Capabilities;
use clifacet::prompt::Terminal;
use clifacet::provenance::{Provenance, Source};
use clifacet::settings::figment::FigmentLoader;
use clifacet::settings::{Entry, Settings};
use clifacet::{App, Failure};
use figment::providers::Serialized;
use figment::value::{Dict, Map, Value};
use figment::{Figment, Metadata, Profile, Provider};

/// Helper provider that tags its values with a file source for testing provenance.
struct FileProvider {
    path: PathBuf,
    profile: Profile,
    dict: Dict,
}

impl FileProvider {
    fn new(path: impl Into<PathBuf>, dict: Dict) -> Self {
        Self {
            path: path.into(),
            profile: Profile::Default,
            dict,
        }
    }

    fn with_profile(path: impl Into<PathBuf>, profile: impl Into<Profile>, dict: Dict) -> Self {
        Self {
            path: path.into(),
            profile: profile.into(),
            dict,
        }
    }
}

impl Provider for FileProvider {
    fn metadata(&self) -> Metadata {
        Metadata::from("file-config", &*self.path)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        let mut map = Map::new();
        map.insert(self.profile.clone(), self.dict.clone());
        Ok(map)
    }
}

fn command_tree() -> Command {
    Command::new("app")
        .no_binary_name(true)
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .action(clap::ArgAction::SetTrue),
        )
        .subcommand(
            Command::new("serve")
                .arg(
                    Arg::new("port")
                        .long("port")
                        .value_parser(clap::value_parser!(u16)),
                )
                .arg(Arg::new("host").long("host"))
                .arg(
                    Arg::new("quiet")
                        .long("quiet")
                        .action(clap::ArgAction::SetTrue),
                )
                .subcommand(
                    Command::new("retry").arg(
                        Arg::new("attempts")
                            .long("attempts")
                            .value_parser(clap::value_parser!(u32)),
                    ),
                ),
        )
        .subcommand(
            Command::new("db").subcommand(
                Command::new("migrate")
                    .arg(
                        Arg::new("dry_run")
                            .long("dry-run")
                            .action(clap::ArgAction::SetTrue),
                    )
                    .arg(Arg::new("target").long("target")),
            ),
        )
}

#[test]
fn figment_maps_nested_command_sections_and_flags() {
    // Criterion #1: FigmentLoader maps nested command configuration sections (e.g. [serve] or
    // [db.migrate]) to corresponding command paths and flags.
    let figment = Figment::new()
        .merge(Serialized::default("serve.port", 8080))
        .merge(Serialized::default("serve.host", "0.0.0.0"))
        .merge(Serialized::default("serve.retry.attempts", 3))
        .merge(Serialized::default("db.migrate.target", "20261118_init"))
        .merge(Serialized::default("db.migrate.dry_run", true));

    let settings = FigmentLoader::new(figment)
        .load_settings()
        .expect("extracts settings from figment");

    let applied = settings
        .apply(command_tree(), &[])
        .expect("sections and flags match the command tree");

    // Test serve subcommand with nested retry
    let matches = applied
        .command()
        .clone()
        .try_get_matches_from(["serve", "retry"])
        .expect("parses with defaults from figment");

    let (_, serve_matches) = matches.subcommand().expect("serve subcommand");
    assert_eq!(serve_matches.get_one::<u16>("port"), Some(&8080));
    assert_eq!(
        serve_matches.get_one::<String>("host").map(String::as_str),
        Some("0.0.0.0")
    );

    let (_, retry_matches) = serve_matches.subcommand().expect("retry subcommand");
    assert_eq!(retry_matches.get_one::<u32>("attempts"), Some(&3));

    // Test nested db migrate subcommand
    let matches = applied
        .command()
        .clone()
        .try_get_matches_from(["db", "migrate"])
        .expect("parses db migrate");

    let (_, db_matches) = matches.subcommand().expect("db subcommand");
    let (_, migrate_matches) = db_matches.subcommand().expect("migrate subcommand");
    assert_eq!(
        migrate_matches
            .get_one::<String>("target")
            .map(String::as_str),
        Some("20261118_init")
    );
    assert!(migrate_matches.get_flag("dry_run"));
}

#[test]
fn precedence_flag_beats_env_beats_figment_beats_default() {
    // Criterion #2: Precedence follows flag > environment > Figment profile/layers > built-in default;
    // CLI flags override Figment values, and Clap's value parsers validate injected defaults.
    let command = Command::new("app").no_binary_name(true).arg(
        Arg::new("port")
            .long("port")
            .env("CLIFACET_TEST_PORT_AC2")
            .default_value("1")
            .value_parser(clap::value_parser!(u16)),
    );

    let figment = Figment::new().merge(Serialized::default("port", 8080));
    let settings = Settings::from_figment(&figment).expect("valid figment");
    let applied = settings.apply(command, &[]).expect("applies");

    // 1. Figment beats built-in default (1)
    let matches = applied
        .command()
        .clone()
        .try_get_matches_from(std::iter::empty::<&str>())
        .expect("parses");
    assert_eq!(matches.get_one::<u16>("port"), Some(&8080));

    // 2. CLI flag (9000) beats Figment (8080)
    let matches = applied
        .command()
        .clone()
        .try_get_matches_from(["--port", "9000"])
        .expect("parses flag");
    assert_eq!(matches.get_one::<u16>("port"), Some(&9000));

    // 3. Clap's value parser validates injected defaults
    let bad_figment = Figment::new().merge(Serialized::default("port", 99999_u32));
    let bad_settings = Settings::from_figment(&bad_figment).expect("settings extracted");
    let command_to_fail = Command::new("app").no_binary_name(true).arg(
        Arg::new("port")
            .long("port")
            .value_parser(clap::value_parser!(u16)),
    );
    let error = bad_settings
        .apply(command_to_fail, &[])
        .expect_err("99999 exceeds u16");
    assert!(
        error.message().contains("99999") || error.message().contains("out of range"),
        "error message: {}",
        error.message()
    );
}

// Function command for Criterion #3
#[clifacet::command]
fn test_serve(
    listen_port: u16,
    host: String,
    #[arg(long, help = "Number of retries", default_value = "3")] retry_count: u32,
) -> Result<String, Failure> {
    Ok(format!("{host}:{listen_port} (retry {retry_count})"))
}

#[test]
fn serde_derive_and_direct_extraction_on_command_function_input() {
    // Criterion #3: #[clifacet::command] on a function derives serde::Deserialize and
    // serde::Serialize on the generated *Input struct when the serde feature is active,
    // enabling direct figment.extract_inner::<Input>(command).

    // Test with snake_case key
    let figment_snake = Figment::new()
        .merge(Serialized::default("test_serve.listen_port", 8080))
        .merge(Serialized::default("test_serve.host", "127.0.0.1"))
        .merge(Serialized::default("test_serve.retry_count", 5));

    let input: TestServeInput = figment_snake
        .extract_inner("test_serve")
        .expect("extracts directly into TestServeInput via serde");
    assert_eq!(input.listen_port, 8080);
    assert_eq!(input.host, "127.0.0.1");
    assert_eq!(input.retry_count, 5);

    // Test with kebab-case key aliases
    let mut dict = Dict::new();
    dict.insert("listen-port".into(), Value::from(9000));
    dict.insert("host".into(), Value::from("0.0.0.0"));
    dict.insert("retry-count".into(), Value::from(10));
    let mut root = Dict::new();
    root.insert("test_serve".into(), Value::from(dict));

    let figment_kebab = Figment::new().merge(FileProvider::new("test.toml", root));
    let input_kebab: TestServeInput = figment_kebab
        .extract_inner("test_serve")
        .expect("deserializes kebab-case aliases");
    assert_eq!(input_kebab.listen_port, 9000);
    assert_eq!(input_kebab.host, "0.0.0.0");
    assert_eq!(input_kebab.retry_count, 10);
}

#[test]
fn key_normalization_resolves_kebab_and_snake_case() {
    // Criterion #4: Key normalization transparently resolves kebab-case CLI flags and
    // snake_case config keys (e.g. listen-port <-> listen_port).

    // CLI flag is kebab-case: `--listen-port`
    let command = Command::new("app")
        .no_binary_name(true)
        .arg(
            Arg::new("listen_port")
                .long("listen-port")
                .value_parser(clap::value_parser!(u16)),
        )
        .arg(Arg::new("db_host").long("db_host"));

    // Config key is snake_case: `listen_port`
    let figment = Figment::new()
        .merge(Serialized::default("listen_port", 8080))
        .merge(Serialized::default("db-host", "pg.example.com"));

    let settings = Settings::from_figment(&figment).expect("valid figment");
    let applied = settings.apply(command, &[]).expect("normalizes keys");

    let matches = applied
        .command()
        .clone()
        .try_get_matches_from(std::iter::empty::<&str>())
        .expect("parses successfully");

    assert_eq!(matches.get_one::<u16>("listen_port"), Some(&8080));
    assert_eq!(
        matches.get_one::<String>("db_host").map(String::as_str),
        Some("pg.example.com")
    );

    // If both kebab-case and snake_case versions of the same key are set, it is rejected as set twice
    let duplicate = Settings::new(vec![
        Entry::new(&[], "listen-port", "8080"),
        Entry::new(&[], "listen_port", "9000"),
    ]);
    let err = duplicate.expect_err("refuses duplicate normalized keys");
    assert!(err.message().contains("set twice"), "{}", err.message());
}

#[test]
fn provenance_records_file_source_with_path_attribution() {
    // Criterion #5: Provenance correctly records Source::File with path attribution when
    // values are injected via Figment file providers.
    let path = Path::new("config/production.toml");
    let mut dict = Dict::new();
    dict.insert("port".into(), Value::from(8443));
    dict.insert("host".into(), Value::from("api.example.com"));

    let figment = Figment::new().merge(FileProvider::new(path, dict));
    let settings = Settings::from_figment(&figment).expect("settings load");

    let command = Command::new("app")
        .no_binary_name(true)
        .arg(
            Arg::new("port")
                .long("port")
                .value_parser(clap::value_parser!(u16)),
        )
        .arg(Arg::new("host").long("host"));

    let applied = settings.apply(command, &[]).expect("applies");

    // Case 1: Value comes from file -> Source::File
    let matches = applied
        .command()
        .clone()
        .try_get_matches_from(std::iter::empty::<&str>())
        .expect("matches without flags");

    let mut provenance = Provenance::from_matches(applied.command(), &matches);
    applied.clone().claim(&mut provenance, &matches);

    assert_eq!(provenance.source_of("port"), Some(Source::File));
    assert_eq!(provenance.source_of("host"), Some(Source::File));

    // Location describes the path
    let entry = settings.get(&[], "port").expect("entry present");
    assert_eq!(
        entry.location().expect("has location").to_string(),
        "config/production.toml"
    );

    // Case 2: Flag overrides file -> Source::Argv
    let overridden = applied
        .command()
        .clone()
        .try_get_matches_from(["--port", "9000"])
        .expect("flag parses");

    let mut provenance = Provenance::from_matches(applied.command(), &overridden);
    applied.claim(&mut provenance, &overridden);

    assert_eq!(provenance.source_of("port"), Some(Source::Argv));
    assert_eq!(provenance.source_of("host"), Some(Source::File));
}

#[clifacet::command(input = PromptInput)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[command(name = "prompt-app", no_binary_name = true)]
struct PromptCmd {
    /// Server port.
    #[clifacet(long, ask = "Server port", form)]
    port: Option<u16>,

    /// Host name.
    #[clifacet(long, form)]
    host: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptInput {
    port: Option<u16>,
    host: Option<String>,
}

struct RunPrompt;

impl clifacet::Command for RunPrompt {
    type Input = PromptInput;
    type Output = String;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        Ok(format!(
            "{}:{}",
            input.host.unwrap_or_default(),
            input.port.unwrap_or_default()
        ))
    }
}

#[test]
fn interactive_prompts_and_tui_receive_figment_defaults() {
    // Criterion #6: Interactive prompts and TUI forms receive Figment-configured defaults
    // as initial values.
    let figment = Figment::new()
        .merge(Serialized::default("port", 8080))
        .merge(Serialized::default("host", "localhost"));

    let mut out = Vec::new();
    let mut err = Vec::new();

    // In interactive mode: since `port` is provided by Figment, it is NOT prompted!
    // An empty prompter stream would fail if it tried to ask a question.
    let code = App::<PromptCmd>::new()
        .command(&RunPrompt)
        .figment(figment)
        .run_with(
            std::iter::empty::<std::ffi::OsString>(),
            Capabilities::terminal(),
            || Box::new(Terminal::stdio()),
            &mut out,
            &mut err,
        );

    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));
    let stdout = String::from_utf8_lossy(&out);
    assert_eq!(stdout.trim(), "localhost:8080");
}

#[test]
fn figment_profile_selection_and_layered_overrides() {
    // Criterion #7: Complete test suite verifying Figment integration, layered overrides,
    // profile selection.
    let mut default_dict = Dict::new();
    default_dict.insert("port".into(), Value::from(8080));
    default_dict.insert("host".into(), Value::from("127.0.0.1"));

    let mut prod_dict = Dict::new();
    prod_dict.insert("port".into(), Value::from(443));
    prod_dict.insert("host".into(), Value::from("0.0.0.0"));

    let figment = Figment::new()
        .merge(FileProvider::with_profile(
            "default.toml",
            Profile::Default,
            default_dict,
        ))
        .merge(FileProvider::with_profile(
            "prod.toml",
            "production",
            prod_dict,
        ))
        .select("production");

    let settings = Settings::from_figment(&figment).expect("settings load");
    let command = Command::new("app")
        .no_binary_name(true)
        .arg(
            Arg::new("port")
                .long("port")
                .value_parser(clap::value_parser!(u16)),
        )
        .arg(Arg::new("host").long("host"));

    let applied = settings.apply(command, &[]).expect("applies");
    let matches = applied
        .command()
        .clone()
        .try_get_matches_from(std::iter::empty::<&str>())
        .expect("parses");

    // Profile 'production' overrides default profile port 8080 -> 443
    assert_eq!(matches.get_one::<u16>("port"), Some(&443));
    assert_eq!(
        matches.get_one::<String>("host").map(String::as_str),
        Some("0.0.0.0")
    );
}

#[clifacet::command(input = AppServeInput)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[command(name = "serve-app", no_binary_name = true)]
struct AppServeCmd {
    /// Port number.
    #[clifacet(long)]
    port: u16,

    /// Host name.
    #[clifacet(long)]
    host: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AppServeInput {
    port: u16,
    host: String,
}

struct AppServeHandler;

impl clifacet::Command for AppServeHandler {
    type Input = AppServeInput;
    type Output = String;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        Ok(format!("listening on {}:{}", input.host, input.port))
    }
}

#[test]
fn app_figment_end_to_end_runner() {
    // Verifies App::figment runs end-to-end through the runner
    let figment = Figment::new()
        .merge(Serialized::default("port", 3000))
        .merge(Serialized::default("host", "0.0.0.0"));

    let app = App::<AppServeCmd>::new()
        .command(&AppServeHandler)
        .figment(figment);

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app.run_with(
        std::iter::empty::<std::ffi::OsString>(),
        Capabilities::headless(),
        || Box::new(Terminal::stdio()),
        &mut out,
        &mut err,
    );

    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&err));
    let stdout = String::from_utf8_lossy(&out);
    assert_eq!(stdout.trim(), "listening on 0.0.0.0:3000");
}

#[cfg(feature = "tui")]
#[test]
fn tui_form_receives_figment_configured_defaults() {
    use clifacet::tui::FormState;

    let figment = Figment::new()
        .merge(Serialized::default("serve.port", 8080))
        .merge(Serialized::default("serve.host", "0.0.0.0"));

    let settings = Settings::from_figment(&figment).expect("settings load");
    let applied = settings
        .apply(command_tree(), &[])
        .expect("applies to command tree");

    let serve_cmd = applied
        .command()
        .find_subcommand("serve")
        .expect("serve subcommand");

    let form = FormState::generated(serve_cmd, &["port", "host"])
        .expect("generated form from serve command");

    let fields = form.fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].value(), "8080");
    assert_eq!(fields[1].value(), "0.0.0.0");
}
