import { createReadStream } from "node:fs";
import { access } from "node:fs/promises";
import { createServer } from "node:http";
import { extname, isAbsolute, join, normalize, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { dirname } from "node:path";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const dist = join(root, "dist");
const port = Number(process.env.PORT ?? 5173);

const mime = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".css", "text/css; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".svg", "image/svg+xml"],
  [".png", "image/png"],
]);

function resolveAsset(url) {
  const path = decodeURIComponent(new URL(url, "http://localhost").pathname);
  const relative = path === "/" ? "index.html" : path.slice(1);
  const candidate = normalize(join(dist, relative));
  const outside = relativePathOutside(dist, candidate);
  return outside ? null : candidate;
}

function relativePathOutside(root, candidate) {
  const rel = relative(root, candidate);
  return rel.startsWith("..") || isAbsolute(rel);
}

createServer(async (request, response) => {
  const asset = resolveAsset(request.url ?? "/");
  if (!asset) {
    response.writeHead(403);
    response.end("forbidden");
    return;
  }
  try {
    await access(asset);
  } catch {
    response.writeHead(404);
    response.end("not found");
    return;
  }
  response.writeHead(200, {
    "Content-Type": mime.get(extname(asset)) ?? "application/octet-stream",
    "X-Content-Type-Options": "nosniff",
  });
  createReadStream(asset).pipe(response);
}).listen(port, "127.0.0.1", () => {
  console.log(`Vesty Web UI demo serving http://localhost:${port}`);
});
