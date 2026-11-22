import Link from "fumadocs-core/link";
import { buttonVariants } from "fumadocs-ui/components/ui/button";
import { HomeLayout } from "fumadocs-ui/layouts/home";

export default function HomePage() {
  return (
    <HomeLayout>
      <main className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-4xl flex-col justify-center gap-10 px-6 py-16">
        <header className="flex flex-col gap-4">
          <div className="flex items-center gap-2">
            <span className="rounded-full bg-fd-primary/10 px-3 py-1 text-xs font-semibold uppercase tracking-wider text-fd-primary">
              v0.10.0
            </span>
            <span className="text-xs text-fd-muted-foreground">
              Rust CLI Framework
            </span>
          </div>
          <h1 className="text-4xl font-extrabold tracking-tight sm:text-5xl">
            Commands as Values, Four Execution Modes
          </h1>
          <p className="text-lg text-fd-muted-foreground">
            Author your command logic once in pure Rust. Execute it noninteractively (flags or structured JSON),
            through conversational questionnaires on stderr, inside a full-screen Ratatui terminal UI, or as a Model Context Protocol tool server.
          </p>
        </header>

        <section className="flex flex-wrap gap-3">
          <Link
            href="/docs"
            className={`${buttonVariants({ variant: "primary" })} px-5 py-2.5 text-base font-medium`}
          >
            Read the Docs
          </Link>
          <Link
            href="/docs/guides/getting-started"
            className={`${buttonVariants({ variant: "outline" })} px-5 py-2.5 text-base font-medium`}
          >
            Getting Started
          </Link>
          <Link
            href="/docs/reference/features"
            className={`${buttonVariants({ variant: "outline" })} px-5 py-2.5 text-base font-medium`}
          >
            Feature Flags
          </Link>
        </section>

        <section className="rounded-xl border border-fd-border bg-fd-card p-6 shadow-sm">
          <h2 className="text-sm font-semibold uppercase tracking-wider text-fd-muted-foreground mb-4">
            Terminal Execution Modes
          </h2>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="flex flex-col gap-2 rounded-lg border border-fd-border/60 bg-fd-accent/30 p-4">
              <span className="font-semibold text-fd-foreground">Noninteractive</span>
              <p className="text-xs text-fd-muted-foreground">
                Flags and positional arguments parsed by Clap. Clean output data on stdout, diagnostics on stderr.
              </p>
              <pre className="mt-auto rounded bg-fd-background p-2 text-xs font-mono text-fd-muted-foreground overflow-x-auto">
                <code>mycli --name ada --repeat 3</code>
              </pre>
            </div>
            <div className="flex flex-col gap-2 rounded-lg border border-fd-border/60 bg-fd-accent/30 p-4">
              <span className="font-semibold text-fd-foreground">Interactive Prompt</span>
              <p className="text-xs text-fd-muted-foreground">
                Conversational questionnaires prompt for missing arguments on stderr with live validation.
              </p>
              <pre className="mt-auto rounded bg-fd-background p-2 text-xs font-mono text-fd-muted-foreground overflow-x-auto">
                <code>mycli --interactive</code>
              </pre>
            </div>
            <div className="flex flex-col gap-2 rounded-lg border border-fd-border/60 bg-fd-accent/30 p-4">
              <span className="font-semibold text-fd-foreground">Full-Screen TUI</span>
              <p className="text-xs text-fd-muted-foreground">
                Ratatui form on the alternate screen buffer, validating inputs on submission.
              </p>
              <pre className="mt-auto rounded bg-fd-background p-2 text-xs font-mono text-fd-muted-foreground overflow-x-auto">
                <code>mycli --tui</code>
              </pre>
            </div>
          </div>
        </section>

        <section className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="rounded-lg border border-fd-border p-4">
            <h3 className="font-medium text-sm mb-1">Declarative Controllers</h3>
            <p className="text-xs text-fd-muted-foreground mb-3">
              Write CLI applications naturally using #[clifacet::app] and #[clifacet::commands] on impl blocks.
            </p>
            <Link
              href="/docs/guides/derive"
              className="text-xs font-medium text-fd-primary hover:underline"
            >
              Explore Derive Guide &rarr;
            </Link>
          </div>
          <div className="rounded-lg border border-fd-border p-4">
            <h3 className="font-medium text-sm mb-1">Agent & Tool Integration</h3>
            <p className="text-xs text-fd-muted-foreground mb-3">
              Serve your CLI as an MCP stdio server or generate machine-readable schemas and agent skills.
            </p>
            <Link
              href="/docs/guides/mcp-server"
              className="text-xs font-medium text-fd-primary hover:underline"
            >
              Learn about MCP Server &rarr;
            </Link>
          </div>
        </section>
      </main>
    </HomeLayout>
  );
}
