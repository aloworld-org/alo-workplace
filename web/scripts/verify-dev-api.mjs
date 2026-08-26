// A local frontend is useful only when the local API it names is alive.
// Vite otherwise starts successfully and turns every refused proxy connection
// into an opaque HTTP 500, which makes a healthy account look as though its
// password is wrong. The full-stack launcher starts the API first; this guard
// waits for it and refuses to serve a misleading login screen if it never
// becomes ready.
const configured = process.env.VITE_DEV_API;
if (configured === undefined || configured.trim() === "") process.exit(0);

let target;
try {
  target = new URL(configured);
} catch {
  console.error(`[dev] VITE_DEV_API is not a valid URL: ${configured}`);
  process.exit(1);
}

const isLoopback = ["localhost", "127.0.0.1", "::1"].includes(target.hostname);
if (!isLoopback) process.exit(0);

const waitMs = Number.parseInt(process.env.VITE_DEV_API_WAIT_MS ?? "30000", 10);
const deadline = Date.now() + (Number.isFinite(waitMs) ? waitMs : 30000);
const readiness = new URL("/health/ready", target);
let lastReason = "no response";

while (Date.now() < deadline) {
  try {
    const response = await fetch(readiness, { signal: AbortSignal.timeout(2000) });
    if (response.ok) {
      const body = await response.json();
      if (body?.status === "ready") {
        console.log(`[dev] API ready at ${target.origin}`);
        process.exit(0);
      }
      lastReason = `health response was ${JSON.stringify(body)}`;
    } else {
      lastReason = `health endpoint returned HTTP ${response.status}`;
    }
  } catch (error) {
    lastReason = error instanceof Error ? error.message : String(error);
  }
  await new Promise((resolve) => setTimeout(resolve, 500));
}

console.error(
  `[dev] Refusing to start the frontend: local API ${target.origin} is not ready (${lastReason}).\n` +
    "Run the complete stack from the repository root with: " +
    "$env:DATABASE_URL='postgres://alo:alo-dev-only@localhost:5432/alo'; ./scripts/dev.ps1 -Action Start",
);
process.exit(1);
