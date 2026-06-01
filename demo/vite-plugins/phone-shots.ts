import type { PluginOption } from "vite";
import { mkdirSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

// Dev-only screenshot relay for phone testing.
//
// Phones can't drop a screenshot onto the dev machine directly, so this
// plugin serves a tiny upload page at `/__shot`: take a normal OS
// screenshot on the phone, open `https://<lan-ip>:5173/__shot` in the
// phone browser, pick the image, and the raw bytes POST back to the dev
// server, which writes them under `demo/shots/` (gitignored). A developer
// — or a coding agent — can then read the screenshot straight off disk to
// see exactly what the phone rendered. See the README "Testing on a
// phone" section.
//
// Only registered for `command === "serve"` (see vite.config.ts), so it
// never ships in a production build.

// `demo/shots/` — sibling of this `vite-plugins/` dir's parent (the demo
// root). Resolved from this module's URL so it's independent of cwd.
const shotsDir = fileURLToPath(new URL("../shots", import.meta.url));

const UPLOAD_PAGE = `<!doctype html><html><head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Send screenshot to PC</title>
<style>
  body { font-family: system-ui, sans-serif; background:#0a0a19; color:#e7e7f5;
         margin:0; padding:2rem; text-align:center; }
  h1 { font-size:1.3rem; }
  p { color:#b8b8d8; }
  input { font-size:1rem; }
  .btn { background:#3b82f6; color:#fff; border:0; border-radius:10px;
         padding:1rem 1.4rem; font-size:1.1rem; font-weight:600; margin-top:1rem; }
  #status { margin-top:1.2rem; min-height:1.4em; font-size:1.05rem; }
</style></head><body>
<h1>Send screenshot to PC</h1>
<p>Take a screenshot on your phone, then choose it below. It saves to the
dev machine under <code>demo/shots/</code> for a developer or agent to read.</p>
<input id="file" type="file" accept="image/*" />
<div><button class="btn" id="send">Upload</button></div>
<div id="status"></div>
<script>
  const file = document.getElementById('file');
  const status = document.getElementById('status');
  document.getElementById('send').onclick = async () => {
    const f = file.files[0];
    if (!f) { status.textContent = 'Choose an image first.'; return; }
    status.textContent = 'Uploading…';
    try {
      const buf = await f.arrayBuffer();
      const r = await fetch('/__shot', {
        method: 'POST',
        headers: { 'x-filename': f.name, 'content-type': f.type || 'application/octet-stream' },
        body: buf,
      });
      status.textContent = r.ok ? ('Uploaded \u2713  (' + f.name + ')') : ('Failed: ' + r.status);
    } catch (e) { status.textContent = 'Error: ' + e; }
  };
</script></body></html>`;

/**
 * Dev-server middleware exposing `/__shot`:
 *  - `GET`  → the upload page above.
 *  - `POST` → save the request body to `demo/shots/<timestamp>-<name>`.
 */
export function phoneShots(): PluginOption {
  return {
    name: "phone-shots",
    configureServer(server) {
      mkdirSync(shotsDir, { recursive: true });
      server.httpServer?.once("listening", () => {
        server.config.logger.info(
          "  \x1b[36m➜\x1b[0m  Phone shots: open \x1b[1m/__shot\x1b[0m on the phone to upload screenshots to demo/shots/",
        );
      });
      server.middlewares.use("/__shot", (req, res) => {
        if (req.method === "GET") {
          res.setHeader("Content-Type", "text/html; charset=utf-8");
          res.end(UPLOAD_PAGE);
          return;
        }
        if (req.method === "POST") {
          const chunks: Buffer[] = [];
          req.on("data", (c) => chunks.push(c as Buffer));
          req.on("end", () => {
            const buf = Buffer.concat(chunks);
            const raw = String(req.headers["x-filename"] ?? "shot.png");
            const safe = raw.replace(/[^a-zA-Z0-9._-]/g, "_");
            const stamp = new Date().toISOString().replace(/[:.]/g, "-");
            const name = `${stamp}-${safe}`;
            writeFileSync(join(shotsDir, name), buf);
            res.statusCode = 200;
            res.setHeader("Content-Type", "text/plain");
            res.end("ok: " + name);
            server.config.logger.info(`[phone-shots] saved demo/shots/${name}`);
          });
          return;
        }
        res.statusCode = 405;
        res.end("method not allowed");
      });
    },
  };
}
