# Evaluation: migrating from `sqlx`/SQLite to Turso (deferred until 1.0)

## Date Created

2026-10-11@15:40

## Status

**Deferred — no implementation.** Turso is not mature enough to replace the current `sqlx` + SQLite data layer.
Revisit once Turso Database reaches a 1.0 release. This document records the findings so the evaluation does not
need to be redone from scratch.

## Summary

The original goal was to keep a **fully local SQLite file** while gaining Turso's improvements (notably
concurrent writes, and better async I/O), driven by the `turso` Rust crate. Two paths were evaluated:

1. **Toasty** (the Tokio-project async ORM) with its native `toasty-driver-turso` driver.
2. **`turso` crate directly** (embedded Turso engine, raw SQL), replacing `sqlx` at the data-access layer.

Both are **technically feasible**, but neither is advisable yet:
- Turso Database and its Rust crate are **pre-1.0**, the release cadence is rapid (0.6 → 0.8 in ~5 months), and the
  feature set is still shifting.
- **Concurrent writes (MVCC)** — the main motivator — is explicitly flagged **experimental / not production-ready**
  by the Turso project, with known gaps (no GC for old row versions, no logical-log recovery on restart, blocking
  checkpoints, no disk spilling).
- Several SQLite behaviors this codebase relies on are **missing or silently ignored** (see gaps below).
- A direct benchmark showed the local-file behavior is close, but Turso is **~2.7x slower** on our single-writer
  insert workload and cannot cap WAL disk usage.

**Decision:** keep `sqlx`/SQLite for now. Re-evaluate at Turso 1.0 against the exit criteria at the end of this
document.

## Context (current state)

- Data layer is built on `sqlx` 0.8. It is **raw SQL** (no ORM): query objects in `src/queries/**` build SQL with
  `sqlx::query`/`query_as` and map rows via `FromRow` into `src/models/*`.
- Pools: `src/database/databases.rs::Databases` holds two `SqlitePool`s (main + session). Controllers acquire a
  connection (`main_pool.acquire()`) and pass `&mut SqliteConnection` down to services/queries
  (e.g. `src/controllers/site/add_site.rs:16`).
- Migrations/seeding/schema-dump are custom, driven by `sqlx::migrate::Migrator` and SQL files in
  `config/databases/main/migrations` (forward `.sql` + `.down.sql`) and `.../seeds`, invoked from
  `src/database/sqlite_database.rs` and `config/scripts/dps_backend_migrate.rs`. `sqlx` tracks applied
  migrations in `_sqlx_migrations`.
- SQLite pragmas are centralized in `src/database/main_database.rs:37` (`SQLITE_PRAGMA_COMMANDS`):
  `journal_mode=WAL`, `foreign_keys=ON`, `synchronous=NORMAL`, `cache_size=-2000`, `page_size=4096`,
  `mmap_size=1048576`, `journal_size_limit=27103364`.
- Tests use `#[dps_backend_db_test]` (crate `dps-backend-test-macros`), which injects `databases` and `main_pool`
  (`SqlitePool`). Unit tests acquire connections directly; integration tests use `create_test_app()` +
  `tower::ServiceExt::oneshot`.

## Findings

### Driver architecture: `sqlx` vs `turso`

`turso` is **not a drop-in driver swap for `sqlx`** — it is a different data-access API:

| Concern | `sqlx` (current) | `turso` crate |
|---|---|---|
| Handle | `SqlitePool` (+ `PoolConnection`) | `Database` (cheaply `Clone`), `Connection` |
| Acquire | `pool.acquire().await?` | `db.connect()?` |
| Concurrency unit | pool size (`max_connections`) | one `Connection` per task; engine handles concurrency |
| Method receiver | `&mut` (`&mut *conn`, `Executor`) | `&self` (`query`, `execute`, `prepare`, `prepare_cached`, `batch`, `transaction`) |
| Row mapping | `FromRow` / `query_as` | manual `row.get::<T>(i)?` |
| Statements returning rows | `execute`/`fetch_*` variants | must use `query` (not `execute`) |
| Migrations | `sqlx::migrate!` + `_sqlx_migrations` | none — must be built/ported |
| Typed support | uuid/chrono features | **none** — `Value` is only `Null/Integer(i64)/Real(f64)/Text/Blob` |
| Toolchain | compiles on repo-pinned Rust 1.88 | requires Rust **>= 1.90** (0.8.2) |

Impact: "no pool, just connections" removes the pool/`&mut` dance, but **all 56 `sqlx`-touching files** would
need rewriting (`src/database/**`, `src/queries/**`, `src/types/database.rs`, plus the test macro/test utils),
and UUID/`chrono` values must be converted by hand.

### Toasty (ORM path)

- Toasty ships a first-class Turso driver supporting a **local file** and opt-in `concurrent_writes()` (MVCC).
- But Toasty is an **ORM** (`#[derive(Model)]`, generated query builders, relations) with its **own migration
  system** — adopting it means migrating models, queries, and the entire migration/seeding/schema-dump tooling,
  plus moving temporal types from `chrono` to `jiff`.
- Toasty 0.10.0 is pre-1.0 with a Rust 1.95 MSRV. More churn and a larger blast radius than the `turso` crate.
- Verdict: heavier than the raw-`turso` path for no benefit to this project.

### Feature gaps (SQLite behaviors we rely on or would miss)

- **`PRAGMA mmap_size` — not supported.** No equivalent; Turso uses its own async pager instead of SQLite's
  mmap VFS.
- **`PRAGMA journal_size_limit` — not supported.** In a benchmark it was **accepted silently but had no effect**
  (no WAL cap). `PRAGMA wal_autocheckpoint` *is* supported and works.
- **`PRAGMA synchronous` — only `OFF` and `FULL`** per the compatibility matrix. `NORMAL` (what we set) was
  accepted without error but is likely a no-op, so durability is not directly comparable.
- **No multi-process access.** Mixed SQLite + Turso multi-process scenarios are unsupported. Relevant if the host
  app, `dps-backend-migrate`, or tooling ever open the same `.db` from separate processes.
- **Result ordering not guaranteed** to match SQLite (add explicit `ORDER BY` where tests assert order).
- **Text must be valid UTF-8** (SQLite permits arbitrary bytes).
- Partial SQL surface: `WITH RECURSIVE` not supported; window functions partial (missing `lag`/`lead`/`ntile`/
  `percent_rank`/`cume_dist`, custom frames); `PRAGMA foreign_key_check` unsupported.

Note: the repo's `docs/manual.md` "Limitations" list (no triggers/views/savepoints) is **stale**; the project
points to `COMPAT.md` as authoritative, which marks those as supported.

### Turbo features that SQLite lacks (upside, if/when we migrate)

- **UUID functions** (`uuid4()`, `uuid4_str()`, `uuid7()`, `uuid_str`/`uuid_blob`, `gen_random_uuid`) — directly
  useful, since we currently generate UUIDv4 strings in Rust. UUIDs stored as `BLOB` by default.
- **Built-in `regexp` extension** (sqlean-compatible), **CDC** (change data capture), **at-rest encryption**
  (experimental), **native async I/O** (io_uring), plus vector search, Tantivy FTS, materialized views, custom
  types/index methods (mostly experimental).
- Concurrent writes via `BEGIN CONCURRENT` + MVCC (row-level conflict detection at commit) — the original goal,
  but experimental.

### Benchmark: WAL growth and throughput (`sqlx` vs `turso`)

Harness: `/tmp/opencode/db-wal-bench` (standalone crate; does not touch this repo's `Cargo.toml`). Workload:
`hits(id INTEGER PK AUTOINCREMENT, val INTEGER)`, 400k rows in 2k-row transactions, single writer,
`journal_mode=WAL`, `cache_size=-2000`, `wal_autocheckpoint=1000`. WAL size sampled after each batch.

| Scenario | Throughput | Peak WAL | Final WAL | Final total | WAL drops | Post-`checkpoint(TRUNCATE)` |
|---|---|---|---|---|---|---|
| `sqlx`, no `journal_size_limit` | 77.8k w/s | 3.93 MiB | 3.93 MiB | 6.28 MiB | 0 | 4.61 MiB |
| `sqlx`, `journal_size_limit=1MiB` | 77.1k w/s | 3.93 MiB | 3.88 MiB | 6.23 MiB | 1 | 4.61 MiB |
| `turso`, no `journal_size_limit` | 28.3k w/s | 3.93 MiB | 3.88 MiB | 6.20 MiB | 1 | 4.58 MiB |
| `turso`, `journal_size_limit=1MiB` (attempted) | 28.2k w/s | 3.93 MiB | 3.88 MiB | 6.20 MiB | 1 | 4.58 MiB |

Observations:
- WAL growth is essentially identical (same file format / same 1000-page autocheckpoint high-water mark).
- `journal_size_limit` truncates the WAL in `sqlx` (sawtooth between ~1 and ~3.9 MiB); `turso` accepts it but
  ignores it, so **no WAL disk cap is possible** on Turso.
- Turso's own autocheckpoint **actually shrinks the WAL file** (drops to near-zero each cycle), unlike default
  SQLite which reuses the file at its high-water mark.
- Explicit `PRAGMA wal_checkpoint(TRUNCATE)` reclaims the WAL on both (must be issued via `query`, not `execute`,
  on turso).
- **Turso is ~2.7x slower** on this single-writer insert workload.
- Turso created no `-shm` sidecar file in single-process mode (only `.db` + `.db-wal`).
- Toolchain note: `turso` 0.8.2 needs Rust >= 1.90; this repo pins 1.88, so the harness was built with
  `cargo +1.99.0`.

## Recommendation

**Do not migrate now.** Keep `sqlx` + SQLite. The current stack is faster on our workload, has the WAL/memory
controls we depend on, supports multi-process access, compiles on the pinned toolchain, and needs no data-layer
rewrite.

Reasons to revisit (in priority order):
1. Turso **1.0**, which the project lists as requiring full SQLite compatibility.
2. **MVCC/concurrent writes** out of experimental status.
3. A WAL disk cap (`journal_size_limit` or equivalent).
4. Clarity on multi-process access.
5. Toolchain/MSRV compatibility with this repo.

## Revisit criteria (exit conditions for re-evaluation)

Before starting any migration work, confirm all of the following against the then-current Turso release:

- [ ] Turso Database and the `turso` crate are **1.0** (or stable), with a compatibility matrix we can audit.
- [ ] MVCC / `BEGIN CONCURRENT` is documented as **production-ready**, including garbage collection and
      crash recovery.
- [ ] `mmap_size`/`journal_size_limit` (or documented equivalents) exist, or we accept losing those controls.
- [ ] `PRAGMA synchronous=NORMAL` is either truly supported or we accept `OFF`/`FULL` only.
- [ ] Multi-process access is supported, or we confirm this project never opens the DB from >1 process.
- [ ] Mixed SQLite/Turso file access is settled (migration path in and back out).
- [ ] `turso` builds on our toolchain (or we agree to bump it).
- [ ] Window functions / `WITH RECURSIVE` / any SQL we use are fully supported.
- [ ] A fresh benchmark run (same harness) shows acceptable throughput for our write profile.

## Future implementation outline (DO NOT START until Turso 1.0 and exit criteria pass)

Provisional phases, ordered to keep each step fully tested before the next. This is a data-layer migration only;
REST routes and DTOs are unchanged.

1. **Data layer** — `src/database/**`:
   - Add `turso` to `Cargo.toml` behind a temporary feature flag or on a branch; remove `sqlx` only at the end.
   - Replace `Databases` (`SqlitePool` fields) with `Database` handles; `main()`/`session()` return `&Database`.
   - Replace `SqliteDatabaseCore` pool construction/pragma setup. Map pragmas: keep `cache_size`, `page_size`,
     `journal_mode`, `foreign_keys`, `wal_autocheckpoint`; drop `mmap_size`, `journal_size_limit`; drop or revisit
     `synchronous`.
   - Wait for / contribute a UUID + temporal type mapping, or add explicit `Value` conversions for `uuid::Uuid`
     and `chrono` (timestamps are already i64 epoch seconds).
2. **Queries** — `src/queries/**` (one file at a time): change signatures from `&mut SqliteConnection` to
   `&Connection`; replace `sqlx::query(...).bind(...)` with `conn.execute/query`; replace `FromRow` mapping with
   manual `row.get` helpers. Add `ORDER BY` where order is asserted.
3. **Services** — `src/services/**`: update connection parameter types; no logic change.
4. **Controllers** — `src/controllers/**`: replace `main_pool.acquire()` with connection acquisition from the new
   `Database`; consider explicit transactions where read-then-write atomicity matters.
5. **Migrations/tooling** — keep the SQL files; port `migrate`/`revert`/`seed`/schema-dump in
   `SqliteDatabaseCore` to run statements via `turso`, and replace `_sqlx_migrations` tracking with our own table
   (or Turso's migration support if it exists by then). Update `config/scripts/dps_backend_migrate.rs`.
6. **Test infrastructure** — update `crates/dps-backend-test-macros` and `src/test_utils/mod.rs` to inject a
   `Database`/`Connection` instead of `main_pool`; drop the `pool_size` argument. Keep `create_test_app()` +
   `oneshot` integration tests as-is.
7. **Benchmark re-run** — rerun `/tmp/opencode/db-wal-bench` (or a repo-local copy) against the new stack and
   record throughput/WAL behavior before/after.
8. **Remove `sqlx`** once all tests pass.

## Open questions

- Do we actually need multiple processes touching the same DB file (host + migrate tooling)? This is the most
  likely hard blocker.
- Is the write throughput of the auth backend high enough to ever need concurrent writes, or is WAL sufficient?
- If the real goal is eventually a **remote/shared** database, should we consider `libsql`/Turso Cloud instead of
  the local embedded engine?

## References

- Benchmark harness: `/tmp/opencode/db-wal-bench` (usage: `cargo +1.99.0 run --release -- [rows] [batch]`;
  sample CSVs in `/tmp/opencode/db-wal-bench/out/`).
- Turso compatibility matrix: `COMPAT.md` in the Turso repo (authoritative).
- Toasty + Turso guide and `toasty-driver-turso` docs (for the ORM path, if reconsidered).
