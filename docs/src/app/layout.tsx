import { RootProvider } from "fumadocs-ui/provider/next";
import type { Metadata } from "next";
import type { ReactNode } from "react";
import "./global.css";

export const metadata: Metadata = {
  title: {
    default: "Clifacet Documentation",
    template: "%s | Clifacet Documentation",
  },
  description:
    "Build a Rust CLI in every terminal mode: noninteractive flags and arguments, conversational questionnaires, or a full-screen Ratatui TUI.",
};

export default function RootLayout({ children }: { readonly children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}
