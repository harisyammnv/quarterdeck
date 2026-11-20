/**
 * Invented containers data for working on the view in a plain browser. Nothing here is read
 * from a Docker engine; names follow the examples used elsewhere in the preview.
 */
import type { ContainerDetail, ContainerLogs, ContainerRow, ContainerStats, ContainersView, EngineSummary } from "./bridge";

const now = Math.floor(Date.now() / 1000);

function row(
  id: string,
  name: string,
  image: string,
  state: ContainerRow["state"],
  opts: Partial<ContainerRow> = {},
): ContainerRow {
  return {
    id,
    name,
    service: null,
    image,
    state,
    status: state === "running" ? "Up 18 minutes" : "Exited (0) 2 hours ago",
    exit_code: null,
    ports: [],
    created_at: now - 7200,
    compose: null,
    ...opts,
  };
}

const port = (host: number | null, container: number) => ({ host, container, proto: "tcp" });

/** `?containers=down|tcp|empty|many` previews the engine states the live app can show. */
function previewMode(): string | null {
  return new URLSearchParams(location.search).get("containers");
}

export function previewContainers(): ContainersView {
  const mode = previewMode();
  if (mode === "down" || mode === "tcp") {
    return {
      ...previewReady(),
      status: mode === "down" ? "not-running" : "unsupported",
      context: mode === "down" ? "desktop" : "remote",
      detail:
        mode === "down"
          ? "Nothing listens on this context's socket."
          : "This Docker context uses a tcp endpoint. Quarterdeck reads local Unix sockets only.",
      engine_version: null,
      kiri_version: null,
      groups: [],
      images: [],
      volumes: [],
      networks: [],
      counts: { containers: 0, running: 0, images: 0, volumes: 0, networks: 0 },
    };
  }
  if (mode === "many") {
    const base = previewReady();
    const other = Array.from({ length: 150 }, (_, i) =>
      row((0x900000000000 + i).toString(16), `batch-worker-${String(i + 1)}`, "example-worker:dev", i % 7 === 0 ? "exited" : "running"),
    );
    return { ...base, other, counts: { ...base.counts, containers: 159, running: 7 + other.filter((r) => r.state === "running").length } };
  }
  if (mode === "empty") {
    return {
      ...previewReady(),
      kiri_version: null,
      context: "desktop",
      groups: [],
      other: [],
      counts: { containers: 0, running: 0, images: 4, volumes: 2, networks: 4 },
    };
  }
  return previewReady();
}

function previewReady(): ContainersView {
  return {
    status: "ready",
    context: "kiri",
    detail: null,
    engine_version: "27.5.1",
    kiri_version: "0.2.1",
    groups: [
      {
        task_id: "login-flake",
        project: "ledger-api",
        worktree: "fm-login-flake",
        state: "working",
        containers: [
          row("0a1b2c3d4e5f", "login-api-1", "ledger-api:dev", "running", {
            service: "api",
            ports: [port(8050, 8050)],
            compose: "fm-login-flake/compose.yaml",
          }),
          row("1b2c3d4e5f60", "login-db-1", "postgres:16", "running", {
            service: "postgres",
            ports: [port(5432, 5432)],
            compose: "fm-login-flake/compose.yaml",
          }),
          row("2c3d4e5f6071", "login-cache-1", "redis:7", "exited", {
            service: "redis",
            compose: "fm-login-flake/compose.yaml",
          }),
        ],
      },
      {
        task_id: "route-perf",
        project: "route-planner",
        worktree: "fm-route-perf",
        state: "working",
        containers: [
          row("3d4e5f607182", "route-frontend", "node:22-slim", "running", {
            ports: [port(3000, 3000)],
            status: "Up 41 minutes",
          }),
          row("4e5f60718293", "route-api", "route-api:dev", "running", {
            ports: [port(8000, 8000)],
            status: "Up 41 minutes",
          }),
          row("5f60718293a4", "route-postgis", "postgis/postgis:16", "running", {
            ports: [port(5433, 5432)],
            status: "Up 41 minutes",
          }),
        ],
      },
      {
        task_id: "export-csv",
        project: "portal-web",
        worktree: "fm-export-csv",
        state: "working",
        containers: [
          row("60718293a4b5", "portal-web", "node:22-slim", "running", {
            ports: [port(3100, 3000)],
            status: "Up About an hour",
          }),
          row("718293a4b5c6", "portal-mail", "axllent/mailpit", "running", {
            ports: [port(8025, 8025)],
            status: "Up About an hour",
          }),
        ],
      },
      {
        task_id: "dash-profile",
        project: "example-app",
        worktree: "fm-dash-profile",
        state: "blocked",
        containers: [
          row("8293a4b5c6d7", "dash", "example-dash:profile", "exited", {
            exit_code: 1,
            ports: [port(null, 8051)],
            status: "Exited (1) 5 minutes ago",
          }),
        ],
      },
      { task_id: "auth-audit", project: "portal-web", worktree: "fm-auth-audit", state: "paused", containers: [] },
    ],
    other: [],
    images: [
      { id: "a1b2c3d4e5f6", tag: "postgres:16", more_tags: 1, size: 438_000_000, created_at: now - 86400 * 20, containers: 1 },
      { id: "b2c3d4e5f6a7", tag: "node:22-slim", more_tags: 0, size: 226_000_000, created_at: now - 86400 * 9, containers: 2 },
      { id: "c3d4e5f6a7b8", tag: "redis:7", more_tags: 0, size: 117_000_000, created_at: now - 86400 * 41, containers: 1 },
      { id: "d4e5f6a7b8c9", tag: "<none>", more_tags: 0, size: 12_000_000, created_at: now - 86400 * 60, containers: 0 },
    ],
    volumes: [
      { name: "login_pgdata", driver: "local", created_at: now - 7200 },
      { name: "route_postgis", driver: "local", created_at: now - 2460 },
    ],
    networks: [
      { id: "e5f6a7b8c9d0", name: "bridge", driver: "bridge", scope: "local", internal: false },
      { id: "f6a7b8c9d0e1", name: "login_default", driver: "bridge", scope: "local", internal: false },
      { id: "a7b8c9d0e1f2", name: "host", driver: "host", scope: "local", internal: false },
      { id: "b8c9d0e1f2a3", name: "none", driver: "null", scope: "local", internal: true },
    ],
    counts: { containers: 9, running: 7, images: 4, volumes: 2, networks: 4 },
    truncated: false,
  };
}

let tick = 0;

export function previewStats(id: string): ContainerStats {
  tick += 1;
  const wave = Math.sin(tick / 2) * 0.8;
  return {
    id,
    cpu_percent: Math.round((3.1 + wave) * 10) / 10,
    memory_bytes: 212_000_000 + tick * 150_000,
    memory_limit: 8_000_000_000,
  };
}

export function previewLogs(id: string): ContainerLogs {
  const at = (ago: number) => now - ago;
  const lines: [number, boolean, string][] = [
    [78, false, "INFO server listening on port 8050"],
    [78, false, "INFO connected to postgres:5432"],
    [39, false, "INFO POST /api/session 200 38ms"],
    [36, false, "INFO POST /login 302 12ms"],
    [36, false, "INFO GET /accounts 200 71ms"],
    [17, true, "WARN slow query on /accounts (412 ms)"],
    [17, false, "INFO POST /api/session 200 41ms"],
    [10, false, "INFO GET /accounts/queue 200 64ms"],
    [5, false, "INFO POST /api/session 200 35ms"],
    [2, false, "INFO GET /healthz 200 2ms"],
  ];
  return {
    id,
    lines: lines.map(([ago, stderr, text]) => ({ at: at(ago), stderr, text })),
    truncated: false,
  };
}

export function previewSummary(): EngineSummary {
  const v = previewContainers();
  return { status: v.status, context: v.context, engine_version: v.engine_version, kiri_version: v.kiri_version };
}

export function previewDetail(id: string): ContainerDetail {
  const row = [...previewContainers().groups.flatMap((g) => g.containers), ...previewContainers().other].find((r) => r.id === id);
  const running = row?.state === "running";
  return {
    id,
    started_at: now - (running ? 1080 : 9000),
    finished_at: running ? null : now - 300,
    exit_code: running ? null : (row?.exit_code ?? 0),
    restart_count: 0,
  };
}
