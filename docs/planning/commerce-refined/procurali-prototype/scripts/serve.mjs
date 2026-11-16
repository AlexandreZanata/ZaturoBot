/**
 * Procurali Prototype — Local Loopback Preview Server (127.0.0.1)
 * Serves the Next.js static export (./out/) with proper Content-Type headers and 404 page.
 */

import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const SCRIPTS_DIR = path.dirname(fileURLToPath(import.meta.url));
const SITE_DIR = path.resolve(SCRIPTS_DIR, "..", "out");
const HOST = "127.0.0.1";
const PORT = Number(process.env.PORT || 4173);

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".ico": "image/x-icon",
  ".xml": "application/xml; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".woff2": "font/woff2",
};

function resolveFilePath(reqUrl) {
  const urlPath = decodeURIComponent((reqUrl || "/").split("?")[0]);
  const safeNormalized = path.posix.normalize(urlPath).replace(/^(\.\.(\/|\\|$))+/, "");
  let candidate = path.join(SITE_DIR, safeNormalized);
  if (!candidate.startsWith(SITE_DIR)) {
    return null;
  }
  if (fs.existsSync(candidate) && fs.statSync(candidate).isDirectory()) {
    candidate = path.join(candidate, "index.html");
  }
  if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
    return candidate;
  }
  return null;
}

const server = http.createServer((req, res) => {
  const filePath = resolveFilePath(req.url);
  if (!filePath) {
    const notFound = path.join(SITE_DIR, "404.html");
    res.writeHead(404, { "Content-Type": "text/html; charset=utf-8" });
    if (fs.existsSync(notFound)) {
      fs.createReadStream(notFound).pipe(res);
    } else {
      res.end("404 — Página não encontrada");
    }
    return;
  }
  const ext = path.extname(filePath).toLowerCase();
  res.writeHead(200, { "Content-Type": MIME[ext] || "application/octet-stream" });
  fs.createReadStream(filePath).pipe(res);
});

server.listen(PORT, HOST, () => {
  console.log(`Procurali preview server running at http://${HOST}:${PORT}/`);
});
