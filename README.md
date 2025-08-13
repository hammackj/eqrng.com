# EQRng.com

A small Rust web service that provides randomized EverQuest-related data (zones, instances, classes, races) plus management utilities and a lightweight admin interface. The project uses `data/data.sql` as the single source of truth for the application's database content and can generate a local SQLite database at `data/zones.db` on startup.

This README is a modernized, concise reference for developers and operators. If you need more in-depth documentation about specific subsystems (migration system, rating transaction logs, etc.), check the `docs/` directory.

---

## Quick overview

- Primary binary: `eq_rng` (web server + API)
- Test binary: `test_db` (database validation)
- Default HTTP port: 3000 (the container exposes 3000 and common setups proxy 80/443 → 3000)
- Data source of truth: `data/data.sql`
- Generated DB file: `data/zones.db` (created from `data/data.sql` on startup)
- Admin UI: optional feature compiled in with the `admin` Cargo feature

---

## Table of contents

- [Requirements](#requirements)
- [Quick start (local)](#quick-start-local)
- [Building](#building)
- [Running](#running)
- [API summary](#api-summary)
- [Data management](#data-management)
- [Admin interface](#admin-interface)
- [Testing](#testing)
- [Docker](#docker)
- [Security notes](#security-notes)
- [Contributing](#contributing)

---

## Requirements

- Rust toolchain (stable)
- Cargo
- SQLite (for inspecting `data/zones.db`)
- Docker & docker-compose (optional, for containerized deployment)

---

## Quick start (local)

1. Ensure `data/data.sql` exists in the repository root.
2. Build and run the server with the admin interface enabled (recommended for local development):
   - Use the admin feature: `cargo run --bin eq_rng --features admin`
3. Visit the API at `http://localhost:3000/` (or use `curl` to exercise endpoints).

Note: On first run the app will create `data/zones.db` from `data/data.sql` if it cannot find an existing DB.

---

## Building

Recommended build patterns:

- Development (with admin):
  - `cargo build`
  - or `cargo build --features admin` (explicit)
- Production (exclude admin):
  - `cargo build --release --no-default-features`
- Build only the main app:
  - `cargo build --package eq_rng`

The repository includes helper scripts (`./utils/build.sh`, `./run_tests.sh`) to simplify common flows; review them before use.

---

## Running

- Development with admin:
  - `cargo run --bin eq_rng --features admin`
- Production (admin disabled):
  - `cargo run --bin eq_rng --no-default-features`
- Run the database test tool:
  - `cargo run --bin test_db --package eq_rng_tests`

The server listens on port 3000 by default. Typical deployments place an HTTP proxy (nginx, Traefik) in front of the container to serve 80/443.

---

## API summary

Below are the primary public endpoints. Use query parameters as documented by the endpoint. Responses are JSON.

- `GET /random_zone`
  - Filter params: `min_level`, `max_level`, `zone_type`, `expansion`, `continent`, `flags` (comma-separated)
- `GET /random_instance`
  - Filter params: `min_level`, `max_level`, `zone_type`, `expansion`, `continent`, `hot_zone`
- `GET /random_class`
  - Optional param: `race` (returns a class compatible with the supplied race)
- `GET /random_race`
  - Returns race with optional gender and image meta
- `GET /version`
  - Returns the running application version

Ratings & notes:

- `GET /zones/:zone_id/rating` — average rating for a zone
- `POST /zones/:zone_id/rating` — submit a rating (logged to transaction log)
- `GET /zones/:zone_id/ratings` — all ratings
- `DELETE /api/ratings/:id` — remove a rating (admin)
- `GET /zones/:zone_id/notes`, `GET /instances/:instance_id/notes` — notes APIs

Links API:

- `GET /api/links`
- `GET /api/links/by-category`
- `GET /api/links/categories`
- `POST /api/links`, `PUT /api/links/:id`, `DELETE /api/links/:id`

This README aims to give a high-level index — consult the source code for exact parameter names and response shapes.

---

## Data management

Primary data strategy:

- `data/data.sql` is the canonical source of truth for the application's content.
- On startup the application checks `data/data.sql` and, if the SQL file is newer than the last applied migration, will recreate the database from it and record a migration timestamp in the `migrations` table.
- The generated SQLite DB lives at `data/zones.db` (do not check generated DBs into version control).

Benefits:

- Human-reviewable diffs in PRs
- Atomic application of data changes
- Simple rollback via VCS

Workflow to update data:

1. Make changes via admin UI or by editing `data/data.sql` directly.
2. If using admin UI, use the "Dump Database" feature to produce `data/data-YYYYMMDD_HHMMSS.sql`.
3. Replace `data/data.sql` with the new dump and commit.

If you edit `data/data.sql` manually, restart the app to trigger a reload.

---

## Rating transaction log

The project uses a file-based rating transaction log to preserve user-submitted rating changes between deployments. Key points:

- Location: `data/rating_transaction.log` (or timestamped exports in `backups/`)
- Format: SQL statements (INSERT/UPDATE/DELETE) that can be applied to the DB
- Management utilities are provided under `utils/` to extract and apply logs
- No public API exists to retrieve or apply the transaction log; it is operated via file-level tooling for safety

Follow the `utils/` scripts when deploying to ensure rating continuity.

---

## Admin interface

- The admin interface is an optional feature controlled by the Cargo `admin` feature flag.
- When compiled out (production build), admin routes are excluded from the binary entirely.
- Admin features include:
  - Zone/instance management
  - Ratings and notes management
  - Link category management
  - Database dump (exports to `data/data-YYYYMMDD_HHMMSS.sql`)
- Important: The admin UI intentionally has no authentication in the project. Only enable it in trusted environments (local/dev). For production, compile without the admin feature.

---

## Testing

- Use the included test utilities: `./run_tests.sh` supports `db`, `build`, and `all` suites.
- Or run the test binary:
  - `cargo run --bin test_db --package eq_rng_tests`
- Tests focus on:
  - Database schema and content validation
  - Read-only integrity checks
  - Build validations for crates in the workspace

See `tests/README.md` for more details about test cases and expected environment.

---

## Docker & deployment

Two primary workflows are supported:

- Production image (admin disabled):
  - `./utils/build.sh production` (builds optimized image without admin)
  - `docker-compose -f docker/docker-compose.yml up -d` to run
- Development image (admin enabled):
  - `./utils/build.sh development`
  - `docker-compose -f docker/docker-compose.dev.yml up -d`

Before deploying:
- Extract `data/rating_transaction.log` if you need to preserve live user ratings between releases.
- Ensure `data/data.sql` in the image is the intended content.

A minimal deployment checklist:
- Review `data/data.sql`
- Build a production image (no admin)
- Extract and preserve rating transaction log prior to update
- Start new container and apply transaction log if needed

---

## Security notes

- Do not enable the admin feature in production images — it contains management routes that are intentionally unauthenticated.
- Treat `data/data.sql` as source code: review in PRs and audit changes.
- The rating transaction log is file-based for portability; protect file access and backups as you would any sensitive data.
- Keep third-party dependencies up to date and monitor for security advisories.

---

## Project structure (high level)

- `src/` — main application code (API handlers, DB setup)
- `data/` — `data.sql`, `zones.db` (generated), transaction logs, JSON helpers like `class_race.json`
- `dist/` — optional frontend build artefacts (a simple test frontend is included)
- `utils/` — deployment and transaction log helper scripts
- `tests/` — test crates and validation tooling
- `Cargo.toml` — workspace configuration

---

## Contributing

- Make all data edits via `data/data.sql` where possible so diffs are reviewable.
- If you need to add or modify endpoints, add unit tests and update documentation.
- Open a PR against the `main` branch with a clear description of changes and any migration steps.

---

## License

This project does not include a license file in this README. Please consult the repository root for licensing information or ask the project owner.

---

If you'd like, I can:
- Add a short example `curl` usage section for each endpoint (as inline commands),
- Create a `DEVELOPMENT.md` or `CONTRIBUTING.md` with step-by-step contributor guidance,
- Or update any specific section to reflect recent changes in the codebase you want highlighted.

Tell me which you'd prefer and I'll update the README accordingly.