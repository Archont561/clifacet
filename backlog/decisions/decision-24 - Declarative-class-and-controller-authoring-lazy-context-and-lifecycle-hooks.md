---
id: decision-24
title: Declarative class and controller authoring, lazy context, and lifecycle hooks
date: '2026-11-19 14:00'
status: accepted
---
## Context

`clifacet` has evolved through multiple authoring ergonomics milestones:
- Decision 13 & 16: Declarative field syntax for argument and option declarations.
- Decision 19 & 21: Runner (`App`), application context threading, folder discovery (`commands!`), and module registry (`#[clifacet::registry]`).
- CLI-52: Standalone command functions (`#[clifacet::command] pub fn cmd(...)`).

Despite these advancements, authoring large, stateful, or deeply nested CLI tools still presents architectural friction:
1. **Context Plumbing:** In function-based commands, every leaf command function must explicitly declare `cx: &AppContext` in its parameter signature. For applications with dozens of commands sharing database connections, API clients, or global logging handles, context drilling becomes repetitive boilerplate.
2. **Eager Context Risk:** CLI applications require strict lazy initialization. Running `mytool --help`, `mytool --version`, or encountering argument syntax errors must never trigger expensive context creation (such as opening remote database pools, reading secret vaults, or configuring disk paths).
3. **Missing Lifecycle Hooks:** Applications currently lack standardized lifecycle interceptors (`before_run`, `after_success`, and especially `after_failure`). When a command fails or aborts, cleanup tasks (e.g., rolling back scratch directories, releasing lockfiles, emitting telemetry, or printing user-remediation hints) must be manually hand-rolled in each command body.
4. **Subcommand Group Ceremony:** Constructing nested subcommand groups currently requires manual dispatch enums, separate folder structures, or explicit struct instantiation glue.
5. **Parser Coupling:** Applications frequently leak `clap` derive details into their public surface, making it difficult to optimize compile times or explore lightweight parsers (such as `argp`).

A controller-centric, class-like authoring model—inspired by the declarative ergonomics of **Pydantic** and **Typer**—unifies global flags, auto-wired context, command dispatch, and lifecycle hooks into one cohesive, developer-friendly architecture.

---

## Decision

Adopt a declarative controller authoring model centered on the `Cli` trait, the `#[clifacet::app]` macro, the `#[clifacet::commands]` `impl` macro, and lifecycle hooks.

### 1. The Core `Cli` Trait

Define a public, extensible `Cli` trait that models the contract between the application, its context, its lifecycle, and the runner:

```rust
pub trait Cli: Sized {
    /// The application context (services, connection pools, configuration).
    type Context;

    /// Lazy context factory.
    /// Guarantee: Never invoked on `--help`, `--version`, or argument parsing errors.
    fn init_context(&self) -> Result<Self::Context, Failure>;

    /// Hook: Runs immediately before command execution starts.
    fn before_run(&self, _cmd: &str, _cx: &Self::Context) -> Result<(), Failure> {
        Ok(())
    }

    /// Hook: Runs after a command successfully finishes.
    fn after_success(&self, _cmd: &str, _cx: &Self::Context) -> Result<(), Failure> {
        Ok(())
    }

    /// Hook: Runs if a command returns an error or panics.
    /// `cx` is an Option: if context creation itself failed, `cx` is None.
    fn after_failure(&self, _cmd: &str, _err: &Failure, _cx: Option<&Self::Context>) {
        // Default: no-op (standard error reporting takes over)
    }
}
```

The runner pipeline (`clifacet::run::<Cli>()`) executes in strict order:
1. Parse CLI arguments.
2. If `--help`, `--version`, or syntax error: render and exit immediately.
3. Invoke `init_context(&self)` lazily. If context construction fails, invoke `after_failure(cmd, err, None)` and exit.
4. Invoke `before_run(cmd, &cx)`.
5. Dispatch command execution with `&self` and `self.cx`.
6. On success: invoke `after_success(cmd, &cx)` and render output.
7. On failure: invoke `after_failure(cmd, &err, Some(&cx))` and render failure report.

### 2. Context Auto-Wiring via `FromCli`

To eliminate manual context instantiation code, provide the `FromCli` trait:

```rust
pub trait FromCli: Sized {
    type Cli;
    fn from_cli(cli: &Self::Cli) -> Result<Self, Failure>;
}
```

When an application struct marks a field `#[clifacet(context)] pub cx: AppContext`, the `#[clifacet::app]` macro automatically synthesizes the `Cli::init_context` implementation by delegating to `<AppContext as FromCli>::from_cli(self)` (or `AppContext::default()` if `Default` is implemented). Alternatively, custom factory functions are supported via `#[clifacet(context = Self::build_context)]`.

### 3. The `#[clifacet::app]` Macro

Decorates the root application struct:
- Declares global CLI options (flags, options, environment variables, default values).
- Mounts subcommand groups via `#[clifacet(subcommand)]`.
- Marks the application context via `#[clifacet(context)]`.
- Emits the underlying parser grammar and derives `clifacet::Cli`.

```rust
#[clifacet::app(name = "mytool", version)]
pub struct MyCli {
    #[clifacet(long, short, help = "Path to config file")]
    pub config: Option<PathBuf>,

    #[clifacet(long, short, help = "Enable verbose output")]
    pub verbose: bool,

    #[clifacet(context)]
    pub cx: AppContext,

    #[clifacet(subcommand)]
    pub users: Users,
}
```

### 4. The `#[clifacet::commands]` Macro on `impl` Blocks

All commands belonging to a controller are authored directly in an `impl` block:
- Every method becomes a CLI command.
- Method parameters map to positional arguments or CLI flags.
- Method doc comments are extracted into `about` (first line) and `long_about` (full description).
- Commands receive `&self`, providing zero-cost, direct access to global CLI flags (`self.verbose`) and initialized context (`self.cx`) without parameter drilling.
- Methods can define or override lifecycle hooks (`after_failure`, `before_run`) directly on the controller.
- Return types implement an extensible `IntoOutcome` trait, accepting `()`, `Result<(), E>`, or typed serializable outputs.

```rust
#[clifacet::commands]
impl MyCli {
    /// Deploy a service to target environment.
    pub fn deploy(&self, service: String, #[clifacet(long)] replicas: u32) -> Result<(), Failure> {
        self.cx.k8s.scale(&service, replicas)?;
        Ok(())
    }

    /// Failure hook for remediation and cleanup:
    fn after_failure(&self, cmd: &str, err: &Failure, cx: Option<&AppContext>) {
        if let Some(cx) = cx {
            cx.rollback();
        }
    }
}
```

### 5. Declarative Subcommand Groups and Context Extractors

Nested command groups are declared as child structs:
- **Field Mounting:** Subcommand groups are declared as fields on the parent struct (`pub users: Users`).
- **Context Extraction:** Child groups declare only the slice of state they require using `#[clifacet(extract)]` or `#[clifacet(context)]`.
- The parent macro automatically synthesizes the child constructor:
  ```rust
  #[clifacet::group]
  pub struct Users {
      #[clifacet(extract)]
      db: DbPool,
  }
  ```
  If `AppContext` implements `AsRef<DbPool>` or `From<&AppContext>`, the child group is instantiated automatically without manual constructor glue.

### 6. Parser Encapsulation

The user-facing API relies exclusively on `clifacet` attributes and the `Cli` trait. The underlying parser (whether `clap` or a lightweight alternative such as `argp`) is an internal implementation detail of the proc-macro desugaring layer, allowing future optimization of compile times and binary footprint without breaking application code.

---

## Consequences

- Authoring ergonomically mirrors modern frameworks (Pydantic, Typer) while preserving Rust's strict compile-time type safety.
- Eliminates context drilling across command signatures: commands access `&self.cx` directly.
- Guaranteed lazy context initialization prevents resource overhead or unwanted side effects on `--help` and `--version`.
- Lifecycle hooks (`before_run`, `after_success`, `after_failure`) provide clean seams for logging, transaction rollbacks, and cleanup.
- Subcommand groups are declared declaratively with zero-boilerplate context extraction.
- Preserves the existing low-level trait seams (`Command`, `Handler`, `Planned`) as internal desugaring targets.
- Establishes Milestone M-18 with tasks CLI-67 through CLI-70.
