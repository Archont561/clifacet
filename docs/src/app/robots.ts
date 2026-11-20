import type { MetadataRoute } from "next";

export const revalidate = false;

export default function robots(): MetadataRoute.Robots {
  const baseUrl = "https://archont561.github.io/clifacet";
  return {
    rules: {
      userAgent: "*",
      allow: "/",
    },
    sitemap: `${baseUrl}/sitemap.xml`,
  };
}
