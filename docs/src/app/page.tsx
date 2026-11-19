import Link from "fumadocs-core/link";
import { buttonVariants } from "fumadocs-ui/components/ui/button";
import { HomeLayout } from "fumadocs-ui/layouts/home";

export default function HomePage() {
  return (
    <HomeLayout>
      <main className="mx-auto flex min-h-[calc(100vh-4rem)] max-w-3xl flex-col justify-center gap-6 px-6 py-16">
        <p className="text-sm font-medium uppercase tracking-wide opacity-70">
          clifacet v0.7.0
        </p>
        <h1 className="text-4xl font-bold tracking-tight">Clifacet Documentation</h1>
        <p className="text-lg opacity-80">
          Build a Rust CLI in every terminal mode: noninteractive flags and arguments,
          conversational questionnaires, or a full-screen Ratatui TUI.
        </p>
        <div className="flex flex-wrap gap-3">
          <Link href="/docs" className={`${buttonVariants({ variant: "primary" })} px-4 py-2`}>
            Read the Docs
          </Link>
          <Link
            href="/docs/guides/getting-started"
            className={`${buttonVariants({ variant: "outline" })} px-4 py-2`}
          >
            Getting Started
          </Link>
          <Link
            href="/docs/concepts/modes"
            className={`${buttonVariants({ variant: "outline" })} px-4 py-2`}
          >
            Execution Modes
          </Link>
          <Link
            href="/docs/reference/command-core"
            className={`${buttonVariants({ variant: "outline" })} px-4 py-2`}
          >
            Command Reference
          </Link>
        </div>
      </main>
    </HomeLayout>
  );
}
