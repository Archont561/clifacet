import { createElement } from "react";
import { icons } from "lucide-react";
import { loader } from "fumadocs-core/source";
import { docs } from "@/.source/server";

/**
 * Fumadocs documentation source loader with Lucide icon mapping.
 */
export const source = loader(docs.toFumadocsSource(), {
  baseUrl: "/docs",
  icon(icon) {
    if (!icon) return;
    if (icon in icons) {
      return createElement(icons[icon as keyof typeof icons]);
    }
  },
});
