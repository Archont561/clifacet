import { DocsLayout } from "fumadocs-ui/layouts/docs";
import type { ReactNode } from "react";
import { source } from "@/lib/source";

export default function Layout({ children }: { readonly children: ReactNode }) {
  return <DocsLayout tree={source.pageTree}>{children}</DocsLayout>;
}
