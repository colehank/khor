import { defineCollection } from "astro:content";
import { glob } from "astro/loaders";
import { z } from "astro/zod";

// One page per file. `order` is the reading order of the list; `summary`
// is the one line the overview shows under the title.
const docs = defineCollection({
  loader: glob({ pattern: "*.md", base: "./src/content/docs" }),
  schema: z.object({
    title: z.string(),
    summary: z.string(),
    order: z.number(),
  }),
});

export const collections = { docs };
