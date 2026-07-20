import { getBackendStatus } from "@/lib/backend-health";

export const dynamic = "force-dynamic";

const foundationChecks = [
  "Rust API boundary",
  "PostgreSQL + PostGIS",
  "Typed health contract",
  "Replay-ready architecture",
];

export default async function Home() {
  const backend = await getBackendStatus();
  const connected = backend.state === "connected";

  return (
    <main className="min-h-screen bg-slate-950 text-slate-100">
      <div className="mx-auto flex min-h-screen max-w-7xl flex-col px-6 py-8 lg:px-10">
        <header className="flex items-center justify-between border-b border-white/10 pb-6">
          <div>
            <p className="font-mono text-xs uppercase tracking-[0.24em] text-cyan-300">
              Flight Tracker AI
            </p>
            <p className="mt-1 text-sm text-slate-400">
              Operations intelligence foundation
            </p>
          </div>
          <div
            className={`flex items-center gap-2 rounded-full border px-3 py-1.5 font-mono text-xs ${
              connected
                ? "border-emerald-400/30 bg-emerald-400/10 text-emerald-300"
                : "border-amber-400/30 bg-amber-400/10 text-amber-300"
            }`}
          >
            <span
              className={`h-2 w-2 rounded-full ${connected ? "bg-emerald-300" : "bg-amber-300"}`}
              aria-hidden="true"
            />
            {connected ? "API connected" : "API degraded"}
          </div>
        </header>

        <section className="grid flex-1 items-center gap-10 py-16 lg:grid-cols-[1.15fr_0.85fr]">
          <div>
            <p className="font-mono text-sm text-cyan-300">FT-001 / FOUNDATION</p>
            <h1 className="mt-5 max-w-3xl text-5xl font-semibold tracking-[-0.04em] text-balance sm:text-6xl">
              A trustworthy operational picture starts with observable systems.
            </h1>
            <p className="mt-6 max-w-2xl text-lg leading-8 text-slate-400">
              The first project slice establishes the typed Rust service, spatial database,
              and live interface boundary that future fleet, weather, and alert workflows
              will build on.
            </p>
          </div>

          <aside className="rounded-2xl border border-white/10 bg-white/[0.04] p-6 shadow-2xl shadow-cyan-950/20">
            <div className="flex items-start justify-between gap-4">
              <div>
                <p className="text-sm font-medium text-slate-200">System foundation</p>
                <p className="mt-1 font-mono text-xs text-slate-500">LIVE DEVELOPMENT CHECK</p>
              </div>
              <span className="rounded-md bg-cyan-300/10 px-2 py-1 font-mono text-xs text-cyan-200">
                M0
              </span>
            </div>

            <dl className="mt-8 space-y-4">
              <div className="flex items-center justify-between border-b border-white/10 pb-4">
                <dt className="text-sm text-slate-400">Backend</dt>
                <dd className="font-mono text-sm text-slate-100">
                  {connected ? backend.health.service : "Unavailable"}
                </dd>
              </div>
              <div className="flex items-center justify-between border-b border-white/10 pb-4">
                <dt className="text-sm text-slate-400">Version</dt>
                <dd className="font-mono text-sm text-slate-100">
                  {connected ? backend.health.version : "—"}
                </dd>
              </div>
              <div className="flex items-start justify-between gap-6">
                <dt className="text-sm text-slate-400">Status detail</dt>
                <dd className="max-w-56 text-right font-mono text-xs leading-5 text-slate-300">
                  {connected ? "Health contract verified" : backend.message}
                </dd>
              </div>
            </dl>

            <ul className="mt-8 grid gap-3 sm:grid-cols-2">
              {foundationChecks.map((check) => (
                <li
                  key={check}
                  className="rounded-lg border border-white/8 bg-slate-950/50 px-3 py-3 text-sm text-slate-300"
                >
                  {check}
                </li>
              ))}
            </ul>
          </aside>
        </section>
      </div>
    </main>
  );
}

