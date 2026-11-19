import { loader } from "fumadocs-core/source";
import { docs } from "@/.source/server";

/**
 * Fumadocs documentation source loader.
 */
export const source = loader(docs.toFumadocsSource(), {
  baseUrl: "/docs",
});
