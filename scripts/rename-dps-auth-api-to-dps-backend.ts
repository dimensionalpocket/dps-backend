/*
 * One-off project-wide rename: dps-auth-api -> dps-backend
 *
 * Also renames orchestrators -> controllers and the test macros crate/macro.
 *
 * Historical docs (docs/llm/plans, ROADMAP, TODO), other one-off scripts,
 * changelog/release metadata, railway/loadtest config, and all
 * DPS_AUTH_API_* / get_auth_api_* / dps-auth-session identifiers are left
 * untouched on purpose.
 *
 * Usage:
 *   bun run scripts/rename-dps-auth-api-to-dps-backend.ts
 */

import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";

const ROOT = join(import.meta.dir, "..");

// --- Content rewrite scope -------------------------------------------------

const FILE_CONTENT_PATHS = [
  "Cargo.toml",
  "Cargo.lock",
  ".gitignore",
  "Dockerfile",
  "start.sh",
  "README.md",
  "AGENTS.md",
  "crates/dps-auth-test-macros/Cargo.toml",
  "crates/dps-auth-test-macros/Cargo.lock",
  "crates/dps-auth-test-macros/src/lib.rs",
  "scripts/run_local_server.rs",
  "config/scripts/dps_auth_api_migrate.rs",
  "config/scripts/migrate.sh",
  "config/databases/main/schema.sql",
  ".github/workflows/test.yml",
  ".github/workflows/docker-build-test.yml",
  ".github/workflows/release.yml",
];

// Whole directory trees that are rewritten in place.
const FILE_CONTENT_DIRS = ["src", "tests"];

// --- Replacements ----------------------------------------------------------
// Order matters: the hyphenated package names first, then identifiers.

const REPLACEMENTS: Array<[RegExp, string]> = [
  [/dps-auth-api/g, "dps-backend"],
  [/dps_auth_api/g, "dps_backend"],
  [/DpsAuthApi/g, "DpsBackend"],
  [/dps_auth_db_test/g, "dps_backend_db_test"],
  [/dps_auth_test_macros/g, "dps_backend_test_macros"],
  [/dps-auth-test-macros/g, "dps-backend-test-macros"],
  [/orchestrators/g, "controllers"],
  [/Orchestrator/g, "Controller"],
];

// --- File/directory moves --------------------------------------------------

const MOVES: Array<[string, string]> = [
  ["src/dps_auth_api.rs", "src/dps_backend.rs"],
  ["src/orchestrators", "src/controllers"],
  ["crates/dps-auth-test-macros", "crates/dps-backend-test-macros"],
  ["config/scripts/dps_auth_api_migrate.rs", "config/scripts/dps_backend_migrate.rs"],
];

function walk(absDir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(absDir, { withFileTypes: true })) {
    const abs = join(absDir, entry.name);
    if (entry.isDirectory()) {
      walk(abs, out);
    } else if (entry.isFile()) {
      out.push(abs);
    }
  }
  return out;
}

function rewrite(relativePath: string) {
  const abs = join(ROOT, relativePath);
  if (!existsSync(abs)) {
    console.log(`skip (missing): ${relativePath}`);
    return;
  }
  const original = readFileSync(abs, "utf8");
  let updated = original;
  for (const [pattern, replacement] of REPLACEMENTS) {
    updated = updated.replace(pattern, replacement);
  }
  if (updated !== original) {
    writeFileSync(abs, updated);
    console.log(`rewrote: ${relativePath}`);
  }
}

function run() {
  const files = new Set<string>(FILE_CONTENT_PATHS);
  for (const dir of FILE_CONTENT_DIRS) {
    const abs = join(ROOT, dir);
    if (existsSync(abs)) {
      for (const file of walk(abs)) {
        files.add(file.slice(ROOT.length + 1));
      }
    }
  }

  for (const file of files) {
    rewrite(file);
  }

  for (const [from, to] of MOVES) {
    const fromAbs = join(ROOT, from);
    const toAbs = join(ROOT, to);
    if (!existsSync(fromAbs)) {
      console.log(`move skip (missing): ${from}`);
      continue;
    }
    mkdirSync(dirname(toAbs), { recursive: true });
    if (existsSync(toAbs)) {
      rmSync(toAbs, { recursive: true, force: true });
    }
    renameSync(fromAbs, toAbs);
    console.log(`moved: ${from} -> ${to}`);
  }
}

run();
