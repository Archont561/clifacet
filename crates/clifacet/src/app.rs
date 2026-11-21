//! The runner: one call that owns an application's pipeline (decision-19 item 1).
//!
//! Every application used to write the same `main` by hand — parse, settle the mode,
//! ask, validate, run or preview, write the output, map the exit code. [`App`] is that
//! pipeline once, so `main.rs` is one line:
//!
//! ```no_run
//! # use clifacet::{Command, Failure, Globals};
//! #[clifacet::command(input = Greeting)]
//! #[derive(Clone)]
//! #[command(name = "greet")]
//! struct Greet {
//!     #[clifacet(long, fill = "world")]
//!     name: Option<String>,
//!     #[command(flatten)]
//!     #[clifacet(skip)]
//!     globals: Globals,
//! }
//!
//! struct Greeting {
//!     name: String,
//! }
//!
//! impl Command for Greet {
//!     type Input = Greeting;
//!     type Output = String;
//!
//!     fn execute(&self, input: Greeting) -> Result<String, Failure> {
//!         Ok(format!("hello {}", input.name))
//!     }
//! }
//!
//! fn main() -> std::process::ExitCode {
//!     clifacet::run::<Greet>()
//! }
//! ```
//!
//! The pipeline, in order, and what each step costs when it stops the run:
//!
//! | Step | Stops with | Exit code |
//! | --- | --- | --- |
//! | parse | `clap`'s help or version, on the data stream | success |
//! | parse | a usage error, on the diagnostic stream | [`ExitPolicy::usage`] |
//! | settle the mode | a [`Refusal`](crate::policy::Refusal), as a validation failure | [`ExitPolicy::validation`] |
//! | ask | a declined question | [`ExitPolicy::cancelled`] |
//! | validate | the bridge's refusal | [`ExitPolicy::validation`] |
//! | run, or preview with `--dry-run` | the command's failure | its kind's code |
//! | render | a data stream that cannot be written | [`ExitPolicy::execution`] |
//!
//! Each step is the library's existing seam — [`Policy::resolve`], [`Session`],
//! [`Collect`], [`Facet::validate`], [`Command`] and [`report::render`] — so the runner
//! adds an order, not a second implementation of any of them.
//!
//! [`run_with`](App::run_with) is the same pipeline with the process taken out: the
//! arguments, the capabilities, the frontend and both streams are parameters, which is
//! how a test runs an application without a terminal or the process environment.

use std::ffi::OsString;
use std::fmt::Display;
use std::io::Write;
use std::process::ExitCode;
use std::rc::Rc;

use clap::{ArgMatches, Parser};

use crate::command::{Command, Handler};
use crate::contract::ExitPolicy;
use crate::facet::Facet;
use crate::failure::Failure;
use crate::policy::{Capabilities, Mode, Policy, Request};
use crate::prompt::{Collect, Lazy, Prompter, Session};
use crate::report;
use crate::settings;

/// The options every application gets: `--yes` and `--dry-run`.
///
/// Flatten it into the root command, beside the application's own options:
///
/// ```
/// #[derive(clap::Parser)]
/// struct Cli {
///     /// The application's own global option.
///     #[arg(long, global = true)]
///     verbose: bool,
///
///     #[command(flatten)]
///     globals: clifacet::Globals,
/// }
///
/// let cli = <Cli as clap::Parser>::try_parse_from(["app", "--dry-run", "--verbose"]).unwrap();
/// assert!(cli.globals.dry_run && cli.verbose);
/// ```
///
/// Both are `global`, so they are accepted after a subcommand too. They decide how a run
/// goes, not what it asks for, which is why an application keeps them out of its input
/// with `#[clifacet(skip)]`. The runner reads them from `clap`'s matches by their ids,
/// `yes` and `dry_run`, so an application that declared its own flags with those ids
/// before this type existed keeps working unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq, clap::Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct Globals {
    /// Answer every confirmation with yes instead of asking.
    #[arg(long, global = true)]
    pub yes: bool,

    /// Show what would run, and run nothing.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Output structured JSON instead of human-readable text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Output the machine-readable CLI JSON schema.
    #[cfg(feature = "schema")]
    #[arg(long, global = true)]
    pub schema: bool,

    /// Read settings from this file instead of the default location.
    ///
    /// Recognised because [`App::settings`] reads it before the parse; declared here so
    /// `clap` accepts it too, and so an application that never calls `settings` gets a
    /// flag that parses rather than one that fails as unknown.
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<std::path::PathBuf>,

    /// Read no settings file at all, ignoring `--config`, the environment and the default
    /// location.
    #[arg(long, global = true, conflicts_with = "config")]
    pub no_config: bool,
}

impl Globals {
    /// The global options as `clap` matched them; an id the command lacks reads as off.
    fn matched(matches: &ArgMatches) -> Self {
        let on = |id: &str| matches!(matches.try_get_one::<bool>(id), Ok(Some(true)));
        Self {
            yes: on("yes"),
            dry_run: on("dry_run"),
            json: on("json"),
            #[cfg(feature = "schema")]
            schema: on("schema"),
            config: matches
                .try_get_one::<std::path::PathBuf>("config")
                .ok()
                .flatten()
                .cloned(),
            no_config: on("no_config"),
        }
    }
}

/// How a command's output is written on the data stream.
///
/// The runner calls it once, with the output of a successful run. [`Plain`] is the
/// default; a closure taking the output and the stream is a renderer too, which is how
/// one output type is written two ways — say, for people and as JSON:
///
/// ```
/// use std::io::Write;
/// use clifacet::Render;
///
/// let json = |count: usize, out: &mut dyn Write| writeln!(out, "{{\"count\":{count}}}");
/// let mut out = Vec::new();
/// json.render(3, &mut out).unwrap();
///
/// assert_eq!(out, b"{\"count\":3}\n");
/// ```
///
/// A failure is never rendered here: it goes through [`report::render`], on the
/// diagnostic stream, whatever the renderer.
pub trait Render<T> {
    /// Writes `output` on `out`.
    ///
    /// # Errors
    ///
    /// Returns the stream's error. The runner reports it as an execution failure: a run
    /// that cannot write its data has nowhere to report success.
    fn render(&self, output: T, out: &mut dyn Write) -> std::io::Result<()>;

    /// Writes `output` as structured JSON on `out`.
    ///
    /// By default delegates to [`render`](Self::render).
    ///
    /// # Errors
    ///
    /// Returns the stream's error.
    fn render_json(&self, output: T, out: &mut dyn Write) -> std::io::Result<()> {
        self.render(output, out)
    }
}

/// The default renderer: an output's [`Display`], then a newline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Plain;

impl<T: Display> Render<T> for Plain {
    fn render(&self, output: T, out: &mut dyn Write) -> std::io::Result<()> {
        writeln!(out, "{output}")
    }

    fn render_json(&self, output: T, out: &mut dyn Write) -> std::io::Result<()> {
        #[cfg(feature = "serde")]
        {
            let text = output.to_string();
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                serde_json::to_writer_pretty(&mut *out, &value).map_err(std::io::Error::other)?;
                return writeln!(out);
            }
            serde_json::to_writer_pretty(&mut *out, &text).map_err(std::io::Error::other)?;
            writeln!(out)
        }
        #[cfg(not(feature = "serde"))]
        {
            writeln!(out, "{output}")
        }
    }
}

pub use crate::cli::{Json, JsonRenderer};

#[cfg(feature = "serde")]
impl<T: serde::Serialize> Render<T> for Json<()> {
    fn render(&self, output: T, out: &mut dyn Write) -> std::io::Result<()> {
        serde_json::to_writer_pretty(&mut *out, &output).map_err(std::io::Error::other)?;
        writeln!(out)
    }

    fn render_json(&self, output: T, out: &mut dyn Write) -> std::io::Result<()> {
        self.render(output, out)
    }
}

impl<T, F> Render<T> for F
where
    F: Fn(T, &mut dyn Write) -> std::io::Result<()>,
{
    fn render(&self, output: T, out: &mut dyn Write) -> std::io::Result<()> {
        self(output, out)
    }

    fn render_json(&self, output: T, out: &mut dyn Write) -> std::io::Result<()> {
        self(output, out)
    }
}

/// Which command a run executes.
enum Chosen<'c, Cli, C> {
    /// Built from the parsed command line — the command line *is* the command.
    Parsed(fn(&Cli) -> C),
    /// Supplied by the application, and the same for every run.
    Given(&'c C),
}

/// An application's pipeline, configured once and run against a command line.
///
/// `Cli` is the root `clap` command. By default the parsed `Cli` is also the command
/// that runs, as a declared tree runs a clone of itself (decision-14); [`command`]
/// runs another command with the same input instead. [`policy`], [`exit`],
/// [`mode_from`] and [`render`] replace the defaults, which are prompts when a terminal
/// can host them ([`Policy::up_to`]`(Mode::Interactive)`), [`ExitPolicy::standard`],
/// no mode requested, and [`Plain`].
///
/// [`command`]: App::command
/// [`policy`]: App::policy
/// [`exit`]: App::exit
/// [`mode_from`]: App::mode_from
/// [`render`]: App::render
pub struct App<'c, Cli, C = Cli, R = Plain, Cx = ()> {
    policy: Policy,
    exit: ExitPolicy,
    mode_from: fn(&Cli) -> Request,
    command: Chosen<'c, Cli, C>,
    render: R,
    #[allow(clippy::type_complexity)]
    context: Rc<dyn Fn(&Cli) -> Result<Cx, Failure>>,
    #[allow(clippy::type_complexity)]
    before_run: Option<Rc<dyn Fn(&Cli, &str, &Cx) -> Result<(), Failure>>>,
    #[allow(clippy::type_complexity)]
    after_success: Option<Rc<dyn Fn(&Cli, &str, &Cx) -> Result<(), Failure>>>,
    #[allow(clippy::type_complexity)]
    after_failure: Option<Rc<dyn Fn(&Cli, &str, &Failure, Option<&Cx>)>>,
    settings: Option<Settings>,
    #[cfg(feature = "async")]
    #[allow(clippy::type_complexity)]
    executor: std::sync::Arc<
        dyn Fn(
                crate::r#async::BoxFuture<'static, Result<Box<dyn std::any::Any + Send>, Failure>>,
                crate::r#async::Cancel,
            ) -> Result<Box<dyn std::any::Any + Send>, Failure>
            + Send
            + Sync,
    >,
}

/// Reads the settings file for a given argument list.
///
/// A function of the arguments rather than a value, because `--config` is in them: the file
/// cannot be read until the run says which file it means.
type Settings = Rc<dyn Fn(&[OsString]) -> Result<settings::Settings, crate::failure::Failure>>;

impl<Cli: Clone> App<'_, Cli> {
    /// The pipeline for `Cli`, with the parsed command line as the command that runs.
    #[must_use]
    pub fn new() -> Self {
        Self {
            policy: Policy::up_to(Mode::Interactive),
            exit: ExitPolicy::standard(),
            mode_from: |_| Request::none(),
            command: Chosen::Parsed(Cli::clone),
            render: Plain,
            context: Rc::new(|_| Ok(())),
            before_run: None,
            after_success: None,
            after_failure: None,
            settings: None,
            #[cfg(feature = "async")]
            executor: {
                #[cfg(feature = "tokio")]
                {
                    std::sync::Arc::new(|fut, cancel| {
                        use crate::r#async::Executor as _;
                        crate::r#async::TokioExecutor::current_thread().run(fut, cancel)
                    })
                }
                #[cfg(not(feature = "tokio"))]
                {
                    std::sync::Arc::new(|fut, cancel| {
                        use crate::r#async::Executor as _;
                        crate::r#async::StdExecutor.run(fut, cancel)
                    })
                }
            },
        }
    }
}

impl<C> App<'static, C, C, Plain, C::Context>
where
    C: crate::cli::Cli + Clone,
{
    /// The pipeline for an application implementing [`Cli`](crate::cli::Cli).
    ///
    /// Lazily constructs [`Cli::Context`] using [`Cli::init_context`], and wires
    /// the lifecycle hooks ([`before_run`](crate::cli::Cli::before_run),
    /// [`after_success`](crate::cli::Cli::after_success), and
    /// [`after_failure`](crate::cli::Cli::after_failure)).
    #[must_use]
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub fn for_cli() -> Self {
        Self {
            policy: Policy::up_to(Mode::Interactive),
            exit: ExitPolicy::standard(),
            mode_from: |_| Request::none(),
            command: Chosen::Parsed(C::clone),
            render: Plain,
            context: Rc::new(|cli: &C| cli.init_context()),
            before_run: Some(Rc::new(|cli, cmd, cx| cli.before_run(cmd, cx))),
            after_success: Some(Rc::new(|cli, cmd, cx| cli.after_success(cmd, cx))),
            after_failure: Some(Rc::new(|cli, cmd, err, cx| cli.after_failure(cmd, err, cx))),
            settings: None,
            #[cfg(feature = "async")]
            executor: {
                #[cfg(feature = "tokio")]
                {
                    std::sync::Arc::new(|fut, cancel| {
                        use crate::r#async::Executor as _;
                        crate::r#async::TokioExecutor::current_thread().run(fut, cancel)
                    })
                }
                #[cfg(not(feature = "tokio"))]
                {
                    std::sync::Arc::new(|fut, cancel| {
                        use crate::r#async::Executor as _;
                        crate::r#async::StdExecutor.run(fut, cancel)
                    })
                }
            },
        }
    }
}

impl<Cli: Clone> Default for App<'_, Cli> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'c, Cli: 'static, C, R, Cx: 'static> App<'c, Cli, C, R, Cx> {
    /// How interactive a run may be, and how interactive it is unasked.
    #[must_use]
    pub fn policy(mut self, policy: Policy) -> Self {
        self.policy = policy;
        self
    }

    /// The exit code each outcome costs.
    #[must_use]
    pub fn exit(mut self, exit: ExitPolicy) -> Self {
        self.exit = exit;
        self
    }

    /// The mode the user asked for, read from the parsed command line.
    ///
    /// For an application with its own `--interactive` or `--tui` flags. `--yes` is
    /// added to whatever this returns, so the function decides the mode only.
    #[must_use]
    pub fn mode_from(mut self, requested: fn(&Cli) -> Request) -> Self {
        self.mode_from = requested;
        self
    }

    /// Runs `command` instead of the parsed command line.
    ///
    /// The input is still `Cli`'s, so the parse, the questions and the validation are
    /// the same; only what runs changes. That is how a test observes a run — see
    /// [`harness::Recording`](crate::harness::Recording).
    #[must_use]
    pub fn command<D>(self, command: &D) -> App<'_, Cli, D, R, Cx> {
        App {
            policy: self.policy,
            exit: self.exit,
            mode_from: self.mode_from,
            command: Chosen::Given(command),
            render: self.render,
            context: self.context,
            before_run: self.before_run,
            after_success: self.after_success,
            after_failure: self.after_failure,
            settings: self.settings,
            #[cfg(feature = "async")]
            executor: self.executor,
        }
    }

    /// Writes the output with `render` instead of [`Plain`].
    #[must_use]
    pub fn render<S>(self, render: S) -> App<'c, Cli, C, S, Cx> {
        App {
            policy: self.policy,
            exit: self.exit,
            mode_from: self.mode_from,
            command: self.command,
            render,
            context: self.context,
            before_run: self.before_run,
            after_success: self.after_success,
            after_failure: self.after_failure,
            settings: self.settings,
            #[cfg(feature = "async")]
            executor: self.executor,
        }
    }

    /// How to build the application's context: what its commands depend on.
    ///
    /// The command runs as a [`Handler`] of that context, which every [`Command`] is. The
    /// context is built after validation, on first use — by a question that reads it
    /// ([`Session::context`]) or by the command — and once per run; so `--help`,
    /// `--version`, a usage error, a rejected value or a declined question never build
    /// it. A context that cannot be built is an execution failure, reported as
    /// `cannot prepare the application: {error}`, and nothing runs.
    #[must_use]
    pub fn context<D: 'static, E, F>(self, build: F) -> App<'c, Cli, C, R, D>
    where
        F: Fn() -> Result<D, E> + 'static,
        E: Display,
    {
        App {
            policy: self.policy,
            exit: self.exit,
            mode_from: self.mode_from,
            command: self.command,
            render: self.render,
            context: Rc::new(move |_| {
                build().map_err(|error| {
                    Failure::execution(format!("cannot prepare the application: {error}"))
                })
            }),
            before_run: None,
            after_success: None,
            after_failure: None,
            settings: self.settings,
            #[cfg(feature = "async")]
            executor: self.executor,
        }
    }

    /// How to build the application's context from the parsed command line.
    #[must_use]
    pub fn context_from<D: 'static, E, F>(self, build: F) -> App<'c, Cli, C, R, D>
    where
        F: Fn(&Cli) -> Result<D, E> + 'static,
        E: Display,
    {
        App {
            policy: self.policy,
            exit: self.exit,
            mode_from: self.mode_from,
            command: self.command,
            render: self.render,
            context: Rc::new(move |cli: &Cli| {
                build(cli).map_err(|error| {
                    Failure::execution(format!("cannot prepare the application: {error}"))
                })
            }),
            before_run: None,
            after_success: None,
            after_failure: None,
            settings: self.settings,
            #[cfg(feature = "async")]
            executor: self.executor,
        }
    }

    /// Sets the executor used to drive asynchronous commands synchronously.
    #[cfg(feature = "async")]
    #[must_use]
    pub fn executor(mut self, executor: impl crate::r#async::Executor + 'static) -> Self {
        self.executor = std::sync::Arc::new(move |fut, cancel| executor.run(fut, cancel));
        self
    }

    /// Hook executed immediately before command execution starts.
    #[must_use]
    pub fn before_run<F>(mut self, hook: F) -> Self
    where
        F: Fn(&str, &Cx) -> Result<(), Failure> + 'static,
    {
        let prev = self.before_run.take();
        self.before_run = Some(Rc::new(move |cli, cmd, cx| {
            if let Some(prev) = &prev {
                prev(cli, cmd, cx)?;
            }
            hook(cmd, cx)
        }));
        self
    }

    /// Hook executed after successful command execution.
    #[must_use]
    pub fn after_success<F>(mut self, hook: F) -> Self
    where
        F: Fn(&str, &Cx) -> Result<(), Failure> + 'static,
    {
        let prev = self.after_success.take();
        self.after_success = Some(Rc::new(move |cli, cmd, cx| {
            if let Some(prev) = &prev {
                prev(cli, cmd, cx)?;
            }
            hook(cmd, cx)
        }));
        self
    }

    /// Hook executed when command execution, `before_run`, or context initialization fails.
    #[must_use]
    pub fn after_failure<F>(mut self, hook: F) -> Self
    where
        F: Fn(&str, &Failure, Option<&Cx>) + 'static,
    {
        let prev = self.after_failure.take();
        self.after_failure = Some(Rc::new(move |cli, cmd, err, cx| {
            if let Some(prev) = &prev {
                prev(cli, cmd, err, cx);
            }
            hook(cmd, err, cx);
        }));
        self
    }
}

impl<Cli, C, R, Cx> App<'_, Cli, C, R, Cx>
where
    Cli: Parser + Facet + Collect + Clone + 'static,
    C: Handler<Cx, Input = <Cli as Facet>::Input>,
    R: Render<<C as Handler<Cx>>::Output>,
    Cx: 'static,
{
    /// Runs the process's own invocation and answers with its exit status.
    ///
    /// The arguments, the terminal's capabilities and the frontend are the process's:
    /// the `inquire` dialog with the `interactive` feature, plain lines on the terminal
    /// without it. A data stream that cannot be flushed downgrades success to
    /// [`ExitPolicy::execution`], because the data may not have arrived.
    #[must_use]
    pub fn run(&self) -> ExitCode {
        let mut out = std::io::stdout().lock();
        let mut err = std::io::stderr().lock();
        let code = self.run_with(
            std::env::args_os(),
            Capabilities::detected(),
            process_frontend,
            &mut out,
            &mut err,
        );
        let code = match out.flush().and_then(|()| err.flush()) {
            Ok(()) => code,
            Err(_) => self.exit.execution(),
        };
        ExitCode::from(u8::try_from(code).unwrap_or(1))
    }

    /// Asynchronously runs the process's own invocation and answers with its exit status.
    ///
    /// Executes directly in the ambient asynchronous runtime without blocking threads.
    #[cfg(feature = "async")]
    pub async fn run_async(&self) -> ExitCode {
        let mut out = std::io::stdout();
        let mut err = std::io::stderr();
        let code = self
            .run_with_async(
                std::env::args_os(),
                Capabilities::detected(),
                process_frontend,
                &mut out,
                &mut err,
            )
            .await;
        let code = match out.flush().and_then(|()| err.flush()) {
            Ok(()) => code,
            Err(_) => self.exit.execution(),
        };
        ExitCode::from(u8::try_from(code).unwrap_or(1))
    }

    /// Layers a settings file under the flags and the environment.
    ///
    /// The file is chosen the way a user expects — `--config PATH`, then `<APP>_CONFIG`,
    /// then the platform's config directory — and every entry it holds becomes a `clap`
    /// default, so the precedence is flag > environment > file > built-in default and each
    /// value is parsed by its argument's own parser (see [`settings`](crate::settings)).
    ///
    /// `loader` decides the format, so the library does not choose an application's
    /// configuration language; `crate::settings::toml::Toml` is the one it ships with the
    /// `settings-toml` feature.
    ///
    /// `--help` and `--version` never read the file: they are answered before any parse, so
    /// the defaults they document are the built-in ones and one machine's settings never
    /// leak into another's documentation. A file that cannot be read or applied fails the
    /// run as a rejected invocation, before anything executes.
    #[must_use]
    pub fn settings<L>(mut self, application: &'static str, loader: L) -> Self
    where
        L: crate::settings::Loader + 'static,
    {
        self.settings = Some(Rc::new(move |argv: &[std::ffi::OsString]| {
            let asked = settings::source::Asked::in_argv(argv.iter().cloned());
            asked
                .read(application, &settings::Process, &loader)
                .map_err(|error| Failure::validation(error.message()))
        }));
        self
    }

    /// Layers a [`figment::Figment`] configuration under the flags and environment.
    ///
    /// Tables in Figment correspond to subcommand paths, outermost first, and scalar values
    /// are injected into `clap` as defaults. The precedence is flag > environment >
    /// figment > built-in default.
    ///
    /// `--help` and `--version` never evaluate settings: they are answered before any parse, so
    /// the defaults they document are the built-in ones.
    #[cfg(feature = "settings-figment")]
    #[must_use]
    pub fn figment(mut self, figment: figment::Figment) -> Self {
        self.settings = Some(Rc::new(move |_argv: &[std::ffi::OsString]| {
            settings::figment::FigmentLoader::new(figment.clone())
                .load_settings()
                .map_err(|error| Failure::validation(error.message()))
        }));
        self
    }

    /// Runs `argv` against explicit capabilities, frontend and streams.
    ///
    /// The same pipeline as [`run`](Self::run), with nothing taken from the process: a
    /// test states what the terminal can do, scripts the answers, and reads both
    /// streams. `build_frontend` is called only when the run asks the user something.
    #[allow(clippy::too_many_lines)]
    pub fn run_with<I, T>(
        &self,
        argv: I,
        capabilities: Capabilities,
        build_frontend: impl FnOnce() -> Box<dyn Prompter>,
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> i32
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        #[cfg(feature = "tokio")]
        if tokio::runtime::Handle::try_current().is_ok() {
            let failure = Failure::execution(
                "invoking synchronous App::run() inside an active Tokio runtime is not \
                 supported; use App::run_async() instead",
            );
            return report::render(&program::<Cli>(), &failure, &self.exit, out, err);
        }

        let argv: Vec<OsString> = argv.into_iter().map(Into::into).collect();
        match self.prepare_argv(&argv, out, err) {
            Ok(Some((cli, globals))) => {
                self.run_parsed(cli, globals, capabilities, build_frontend, out, err)
            }
            Ok(None) => self.exit.success(),
            Err(code) => code,
        }
    }

    /// Prepares and parses command-line arguments, handling early flags such as `--help` or `--schema`.
    fn prepare_argv(
        &self,
        argv: &[OsString],
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Result<Option<(Cli, Globals)>, i32> {
        if self.settings.is_some() && asks_for_help_or_version(argv) {
            match <Cli as Facet>::command().try_get_matches_from(argv.iter().cloned()) {
                Ok(_) => {}
                Err(error) => {
                    return Err(report::render(
                        &program::<Cli>(),
                        &error,
                        &self.exit,
                        out,
                        err,
                    ));
                }
            }
        }

        #[cfg(feature = "schema")]
        if asks_for_schema(argv) {
            let cmd = <Cli as Facet>::command();
            let meta = crate::schema::CliMetadata::from_command(&cmd);
            match crate::schema::to_json_schema_pretty(&meta) {
                Ok(json) => {
                    let _ = writeln!(out, "{json}");
                    return Err(self.exit.success());
                }
                Err(failure) => {
                    return Err(report::render(
                        &program::<Cli>(),
                        &failure,
                        &self.exit,
                        out,
                        err,
                    ));
                }
            }
        }

        #[cfg(feature = "skill-minijinja")]
        if let Some(opts) = asks_for_write_skill(argv, &<Cli as Facet>::command()) {
            use crate::skill::SkillRenderer as _;
            let cmd = <Cli as Facet>::command();
            let meta = crate::schema::CliMetadata::from_command(&cmd);
            let renderer = crate::skill::MiniJinjaSkillRenderer::new();
            match renderer.render(&meta).and_then(|rendered| {
                crate::skill::execute_write_skill(&meta.name, &rendered, &opts)
            }) {
                Ok(msg) => {
                    if opts.stdout {
                        let _ = write!(out, "{msg}");
                    } else if !msg.is_empty() {
                        let _ = writeln!(out, "{msg}");
                    }
                    return Err(self.exit.success());
                }
                Err(failure) => {
                    return Err(report::render(
                        &program::<Cli>(),
                        &failure,
                        &self.exit,
                        out,
                        err,
                    ));
                }
            }
        }

        match self.parse_with_settings(argv) {
            Ok((cli, globals)) => Ok(Some((cli, globals))),
            Err(error) => {
                let json = asks_for_json(argv);
                if json && error.use_stderr() {
                    Err(report::render_json(
                        &program::<Cli>(),
                        &error,
                        &self.exit,
                        out,
                        err,
                    ))
                } else {
                    Err(report::render(
                        &program::<Cli>(),
                        &error,
                        &self.exit,
                        out,
                        err,
                    ))
                }
            }
        }
    }

    /// Parses `argv`, with the settings file applied to the command when one is configured.
    ///
    /// Without settings this is [`parse`] and nothing else: an application that never calls
    /// [`settings`](Self::settings) pays for one `Option` check.
    fn parse_with_settings(&self, argv: &[OsString]) -> Result<(Cli, Globals), clap::Error> {
        let Some(read) = &self.settings else {
            return parse::<Cli, _, _>(argv.iter().cloned());
        };
        let settings = match read(argv) {
            Ok(settings) => settings,
            // A settings file that cannot be read is a rejected invocation: nothing ran,
            // and the argument list the user typed was not the problem. Reporting it as a
            // `clap` error keeps one path to the diagnostic stream and one exit code.
            Err(failure) => {
                return Err(clap::Error::raw(
                    clap::error::ErrorKind::ValueValidation,
                    failure.message(),
                ));
            }
        };
        let command = settings::source::declare(<Cli as Facet>::command());
        let applied = match settings.apply(command, <Cli as Facet>::secret_fields()) {
            Ok(applied) => applied,
            Err(error) => {
                return Err(clap::Error::raw(
                    clap::error::ErrorKind::ValueValidation,
                    error.message(),
                ));
            }
        };
        let mut matches = applied
            .into_command()
            .try_get_matches_from(argv.iter().cloned())?;
        let globals = Globals::matched(&matches);
        let cli = Cli::from_arg_matches_mut(&mut matches)
            .map_err(|error| error.format(&mut <Cli as Facet>::command()))?;
        Ok((cli, globals))
    }

    /// Runs an already-parsed command line: every step after parsing.
    ///
    /// For an application that holds a `Cli` it parsed itself. `globals` are passed
    /// rather than read, because a parsed `Cli` no longer has `clap`'s matches to read
    /// them from.
    ///
    /// `globals` is taken by value because this is a published signature: an application
    /// calls it, and a future `Globals` may well be consumed here. Changing it to a
    /// reference to satisfy a lint would be a breaking change for no gain.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "a published signature, kept stable for callers"
    )]
    pub fn run_parsed(
        &self,
        cli: Cli,
        globals: Globals,
        capabilities: Capabilities,
        build_frontend: impl FnOnce() -> Box<dyn Prompter>,
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> i32 {
        let requested = (self.mode_from)(&cli);
        let requested = requested.assuming_yes(requested.assumes_yes() || globals.yes);
        // One context per run, shared by the questions and the command, built on first use.
        let cli_for_cx = cli.clone();
        let builder = Rc::clone(&self.context);
        let context = Rc::new(Lazy::new(Rc::new(move || builder(&cli_for_cx))));
        let outcome = match self.policy.resolve(requested, capabilities) {
            Ok(plan) if plan.mode() == Mode::Tui => self.full_screen(&globals, &context, out, err),
            Ok(plan) => {
                let mut session = Session::start(plan, build_frontend).sharing(Rc::clone(&context));
                cli.collect(&mut session)
                    .and_then(|cli| self.settle(cli, &globals, &context, out))
            }
            Err(refusal) => Err(Failure::validation(refusal.message())),
        };
        self.conclude(outcome, &globals, out, err)
    }

    /// Asynchronously runs `argv` against explicit capabilities, frontend and streams.
    ///
    /// The asynchronous counterpart to [`run_with`](Self::run_with), executing natively
    /// within an existing asynchronous runtime without blocking threads.
    #[cfg(feature = "async")]
    #[allow(clippy::too_many_lines)]
    pub async fn run_with_async<I, T>(
        &self,
        argv: I,
        capabilities: Capabilities,
        build_frontend: impl FnOnce() -> Box<dyn Prompter>,
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> i32
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let argv: Vec<OsString> = argv.into_iter().map(Into::into).collect();
        match self.prepare_argv(&argv, out, err) {
            Ok(Some((cli, globals))) => {
                self.run_parsed_async(cli, globals, capabilities, build_frontend, out, err)
                    .await
            }
            Ok(None) => self.exit.success(),
            Err(code) => code,
        }
    }

    /// Asynchronously runs the parsed command line.
    #[cfg(feature = "async")]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "a published signature, kept stable for callers"
    )]
    pub async fn run_parsed_async(
        &self,
        cli: Cli,
        globals: Globals,
        capabilities: Capabilities,
        build_frontend: impl FnOnce() -> Box<dyn Prompter>,
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> i32 {
        let requested = (self.mode_from)(&cli);
        let requested = requested.assuming_yes(requested.assumes_yes() || globals.yes);
        let cli_for_cx = cli.clone();
        let builder = Rc::clone(&self.context);
        let context = Rc::new(Lazy::new(Rc::new(move || builder(&cli_for_cx))));
        let outcome = match self.policy.resolve(requested, capabilities) {
            Ok(plan) if plan.mode() == Mode::Tui => self.full_screen(&globals, &context, out, err),
            Ok(plan) => {
                let mut session = Session::start(plan, build_frontend).sharing(Rc::clone(&context));
                match cli.collect(&mut session) {
                    Ok(collected_cli) => {
                        self.settle_async(collected_cli, &globals, &context, out)
                            .await
                    }
                    Err(err) => Err(err),
                }
            }
            Err(refusal) => Err(Failure::validation(refusal.message())),
        };
        self.conclude(outcome, &globals, out, err)
    }

    /// Asynchronously settles the run after collection.
    #[cfg(feature = "async")]
    #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
    async fn settle_async(
        &self,
        cli: Cli,
        globals: &Globals,
        context: &Lazy<Cx>,
        out: &mut dyn Write,
    ) -> Result<(), Failure> {
        self.settle(cli, globals, context, out)
    }

    /// Hosts the form generated from `Cli`'s `form` fields, drawn on the diagnostic stream.
    ///
    /// The form produces arguments and `clap` parses them, so a submission reaches the
    /// same validation, command and renderer as a flag run. A refused submission is
    /// shown on the form rather than ending the run, because a form that closes on a
    /// typo makes the user start again. The rendered output is held until the screen
    /// is left, then written on the data stream.
    #[cfg(feature = "tui")]
    fn full_screen(
        &self,
        globals: &Globals,
        context: &Lazy<Cx>,
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> Result<(), Failure> {
        use crate::tui;

        let mut form = tui::FormState::of::<Cli>()?;
        let mut rendered = Vec::new();
        // The globals are borrowed rather than copied: the settings flags mean `Globals` is
        // no longer `Copy`, and a form submission is the same run as the one that opened it.
        let submit = |state: &tui::FormState| -> Result<Vec<String>, String> {
            let argv = std::iter::once(program::<Cli>()).chain(state.arguments());
            let cli = Cli::try_parse_from(argv).map_err(|error| error.to_string())?;
            rendered.clear();
            self.settle(cli, globals, context, &mut rendered)
                .map_err(|failure| failure.message().to_owned())?;
            Ok(Vec::new())
        };
        tui::run_on(&mut *err, &mut form, tui::live_keys(), submit)?;
        out.write_all(&rendered)
            .map_err(|error| Failure::execution(format!("cannot write the output: {error}")))
    }

    /// Without the `tui` feature there is no form to host, so the mode is refused.
    #[cfg(not(feature = "tui"))]
    #[allow(
        clippy::unused_self,
        reason = "the same signature as the `tui` build's method, so the call site is one"
    )]
    fn full_screen(
        &self,
        _globals: &Globals,
        _context: &Lazy<Cx>,
        _out: &mut dyn Write,
        _err: &mut dyn Write,
    ) -> Result<(), Failure> {
        Err(Failure::validation(
            "a TUI is not available: this program was built without clifacet's `tui` feature",
        ))
    }

    /// Validates, then runs or previews, then renders: everything after the questions.
    #[allow(clippy::collapsible_if, clippy::too_many_lines)]
    fn settle(
        &self,
        cli: Cli,
        globals: &Globals,
        context: &Lazy<Cx>,
        out: &mut dyn Write,
    ) -> Result<(), Failure> {
        #[cfg(feature = "schema")]
        if globals.schema {
            let cmd = <Cli as Facet>::command();
            let meta = crate::schema::CliMetadata::from_command(&cmd);
            let json = crate::schema::to_json_schema_pretty(&meta)?;
            writeln!(out, "{json}")
                .map_err(|e| Failure::execution(format!("cannot write the output: {e}")))?;
            return Ok(());
        }

        let cmd = program::<Cli>();
        // The command is taken before validation consumes the command line it is
        // built from — the clone a declared tree keeps of itself.
        let parsed;
        let command = match &self.command {
            Chosen::Parsed(build) => {
                parsed = build(&cli);
                &parsed
            }
            Chosen::Given(command) => *command,
        };
        let cli_ref = cli.clone();
        let input = cli.validate()?;

        let cx = match context.get() {
            Ok(cx) => cx,
            Err(err) => {
                if let Some(hook) = &self.after_failure {
                    hook(&cli_ref, &cmd, &err, None);
                }
                return Err(err);
            }
        };

        if let Some(hook) = &self.before_run {
            if let Err(err) = hook(&cli_ref, &cmd, &cx) {
                if let Some(fail_hook) = &self.after_failure {
                    fail_hook(&cli_ref, &cmd, &err, Some(&cx));
                }
                return Err(err);
            }
        }

        let outcome = if globals.dry_run {
            match Handler::<Cx>::preview(command, input, &cx) {
                Ok(plan) => {
                    let steps = plan.describe();
                    if steps.is_empty() {
                        Ok(())
                    } else if globals.json {
                        #[cfg(feature = "serde")]
                        {
                            let payload = serde_json::json!({
                                "dry_run": true,
                                "steps": plan.steps(),
                            });
                            serde_json::to_writer_pretty(&mut *out, &payload)
                                .map_err(std::io::Error::other)
                                .and_then(|()| writeln!(out))
                                .map_err(|error| {
                                    Failure::execution(format!("cannot write the output: {error}"))
                                })
                        }
                        #[cfg(not(feature = "serde"))]
                        {
                            writeln!(out, "{steps}").map_err(|error| {
                                Failure::execution(format!("cannot write the output: {error}"))
                            })
                        }
                    } else {
                        writeln!(out, "{steps}").map_err(|error| {
                            Failure::execution(format!("cannot write the output: {error}"))
                        })
                    }
                }
                Err(err) => Err(err),
            }
        } else {
            match Handler::<Cx>::handle(command, input, &cx) {
                Ok(output) => {
                    if globals.json {
                        self.render.render_json(output, out).map_err(|error| {
                            Failure::execution(format!("cannot write the output: {error}"))
                        })
                    } else {
                        self.render.render(output, out).map_err(|error| {
                            Failure::execution(format!("cannot write the output: {error}"))
                        })
                    }
                }
                Err(err) => Err(err),
            }
        };

        match outcome {
            Ok(()) => {
                if let Some(hook) = &self.after_success {
                    if let Err(err) = hook(&cli_ref, &cmd, &cx) {
                        if let Some(fail_hook) = &self.after_failure {
                            fail_hook(&cli_ref, &cmd, &err, Some(&cx));
                        }
                        return Err(err);
                    }
                }
                Ok(())
            }
            Err(err) => {
                if let Some(fail_hook) = &self.after_failure {
                    fail_hook(&cli_ref, &cmd, &err, Some(&cx));
                }
                Err(err)
            }
        }
    }

    /// The exit code a finished run answers with, its failure reported if it had one.
    fn conclude(
        &self,
        outcome: Result<(), Failure>,
        globals: &Globals,
        out: &mut dyn Write,
        err: &mut dyn Write,
    ) -> i32 {
        match outcome {
            Ok(()) => self.exit.success(),
            Err(failure) => {
                if globals.json {
                    report::render_json(&program::<Cli>(), &failure, &self.exit, out, err)
                } else {
                    report::render(&program::<Cli>(), &failure, &self.exit, out, err)
                }
            }
        }
    }
}

/// Runs `Cli` as the whole application: the one line a `main` needs.
///
/// The parsed command line is the command, its output is written with [`Plain`], and
/// every other choice is [`App`]'s default; build an [`App`] to change any of them.
#[must_use]
pub fn run<Cli>() -> ExitCode
where
    Cli: Parser + Facet + Collect + Clone + Command<Input = <Cli as Facet>::Input> + 'static,
    <Cli as Command>::Output: Display,
{
    App::<Cli>::new().run()
}

/// Runs `C` as an application implementing [`Cli`](crate::cli::Cli).
#[must_use]
pub fn run_cli<C>() -> ExitCode
where
    C: crate::cli::Cli
        + Parser
        + Facet
        + Collect
        + Clone
        + Handler<C::Context, Input = <C as Facet>::Input>
        + 'static,
    <C as Handler<C::Context>>::Output: Display,
    C::Context: 'static,
{
    App::<C, C, Plain, C::Context>::for_cli().run()
}

/// Runs `C` asynchronously as an application implementing [`Cli`](crate::cli::Cli).
#[cfg(feature = "async")]
pub async fn run_cli_async<C>() -> ExitCode
where
    C: crate::cli::Cli
        + Parser
        + Facet
        + Collect
        + Clone
        + Handler<C::Context, Input = <C as Facet>::Input>
        + 'static,
    <C as Handler<C::Context>>::Output: Display,
    C::Context: 'static,
{
    App::<C, C, Plain, C::Context>::for_cli().run_async().await
}

/// Whether the argument list asks for help or a version, which is answered without a file.
///
/// `--` ends the flags, so anything after it names a value rather than asking a question.
fn asks_for_help_or_version(argv: &[OsString]) -> bool {
    for argument in argv {
        let Some(text) = argument.to_str() else {
            continue;
        };
        match text {
            "--" => return false,
            "--help" | "-h" | "--version" | "-V" => return true,
            _ => {}
        }
    }
    false
}

/// Whether the argument list asks for structured JSON output.
fn asks_for_json(argv: &[OsString]) -> bool {
    for argument in argv {
        let Some(text) = argument.to_str() else {
            continue;
        };
        match text {
            "--" => return false,
            "--json" => return true,
            _ => {}
        }
    }
    false
}

/// Whether the argument list asks for machine-readable CLI JSON schema.
#[cfg(feature = "schema")]
fn asks_for_schema(argv: &[OsString]) -> bool {
    if asks_for_help_or_version(argv) {
        return false;
    }
    for argument in argv {
        let Some(text) = argument.to_str() else {
            continue;
        };
        match text {
            "--" => return false,
            "--schema" | "schema" => return true,
            _ => {}
        }
    }
    false
}

/// Whether the argument list asks for write-skill.
#[cfg(feature = "skill-minijinja")]
fn asks_for_write_skill(
    argv: &[OsString],
    cmd: &clap::Command,
) -> Option<crate::skill::WriteSkillOptions> {
    if cmd.find_subcommand("write-skill").is_some() {
        return None;
    }
    if argv.len() < 2 {
        return None;
    }
    if argv[1].to_str()? != "write-skill" {
        return None;
    }
    if asks_for_help_or_version(argv) {
        return None;
    }
    let mut stdout = false;
    let mut output = None;
    let mut check = false;
    let mut iter = argv[2..].iter();
    while let Some(arg) = iter.next() {
        let Some(text) = arg.to_str() else {
            continue;
        };
        match text {
            "--stdout" => stdout = true,
            "--check" => check = true,
            "-o" | "--output" => {
                if let Some(val) = iter.next() {
                    output = Some(std::path::PathBuf::from(val));
                }
            }
            s if s.starts_with("--output=") => {
                output = Some(std::path::PathBuf::from(&s["--output=".len()..]));
            }
            _ => {}
        }
    }
    Some(crate::skill::WriteSkillOptions {
        stdout,
        output,
        check,
    })
}

/// Parses `argv` into the command line and the global options it carried.
///
/// `clap`'s own two steps, as [`Parser::try_parse_from`] takes them, with the globals
/// read from the matches in between.
fn parse<Cli, I, T>(argv: I) -> Result<(Cli, Globals), clap::Error>
where
    Cli: Parser,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut matches = Cli::command().try_get_matches_from(argv)?;
    let globals = Globals::matched(&matches);
    let cli = Cli::from_arg_matches_mut(&mut matches)
        .map_err(|error| error.format(&mut Cli::command()))?;
    Ok((cli, globals))
}

/// How a diagnostic names the program: the root command's name.
fn program<Cli: Facet>() -> String {
    <Cli as Facet>::command().get_name().to_owned()
}

/// The frontend a real process asks with.
fn process_frontend() -> Box<dyn Prompter> {
    #[cfg(feature = "interactive")]
    let frontend = Box::new(crate::prompt::Dialog::new());
    #[cfg(not(feature = "interactive"))]
    let frontend = Box::new(crate::prompt::Terminal::stdio());
    frontend
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, clap::Parser)]
    #[command(name = "unitapp")]
    struct UnitCli {
        #[arg(long)]
        flag: bool,
    }

    impl Facet for UnitCli {
        type Input = Self;
        fn command() -> clap::Command {
            <Self as clap::CommandFactory>::command()
        }
        fn validate(self) -> Result<Self::Input, Failure> {
            Ok(self)
        }
    }

    impl crate::prompt::Collect for UnitCli {
        fn collect(self, _session: &mut crate::prompt::Session) -> Result<Self, Failure> {
            Ok(self)
        }
    }

    impl crate::Handler<()> for UnitCli {
        type Input = Self;
        type Output = String;
        fn handle(&self, _input: Self::Input, _cx: &()) -> Result<Self::Output, Failure> {
            Ok(format!("flag={}", self.flag))
        }
    }

    #[test]
    fn app_render_and_context_builders() {
        let app = App::<UnitCli>::new()
            .render(Plain)
            .before_run(|_, ()| Ok(()))
            .after_success(|_, ()| Ok(()))
            .after_failure(|_, _, _| ())
            .context(|| Ok::<_, std::convert::Infallible>(()))
            .context_from(|_| Ok::<_, std::convert::Infallible>(()));
        assert_eq!(app.policy.ceiling(), Mode::Interactive);
    }

    #[cfg(feature = "async")]
    #[test]
    fn app_async_builder_and_prepare_argv() {
        let app = App::<UnitCli>::new().executor(crate::r#async::StdExecutor);
        let mut out = Vec::new();
        let mut err = Vec::new();

        let parsed = app.prepare_argv(
            &[OsString::from("unitapp"), OsString::from("--flag")],
            &mut out,
            &mut err,
        );
        assert!(parsed.is_ok());
        let (cli, _globals) = parsed.unwrap().unwrap();
        assert!(cli.flag);

        let code = app.run_parsed(
            cli,
            Globals::default(),
            Capabilities::headless(),
            process_frontend,
            &mut out,
            &mut err,
        );
        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(out).unwrap(), "flag=true\n");
    }

    #[cfg(all(feature = "async", feature = "tokio"))]
    #[test]
    fn app_run_parsed_async_unit() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(async {
            let app = App::<UnitCli>::new();
            let mut out = Vec::new();
            let mut err = Vec::new();
            let code = app
                .run_parsed_async(
                    UnitCli { flag: true },
                    Globals::default(),
                    Capabilities::headless(),
                    process_frontend,
                    &mut out,
                    &mut err,
                )
                .await;
            assert_eq!(code, 0);
            assert_eq!(String::from_utf8(out).unwrap(), "flag=true\n");
        });
    }
}
