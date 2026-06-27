import { cp, mkdir, rm } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const src = join(root, "src");
const dist = join(root, "dist");

await rm(dist, { force: true, recursive: true });
await mkdir(dist, { recursive: true });
await cp(join(src, "index.html"), join(dist, "index.html"));
await cp(join(src, "index.js"), join(dist, "index.js"));

console.log(`built ${dist}`);
