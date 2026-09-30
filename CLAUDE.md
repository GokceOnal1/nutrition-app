# CLAUDE.md

## Project

Single-user, self-hosted fitness and nutrition tracker. Long-term scope: the union of a strength logger (Hevy), endurance log (Strava), adaptive nutrition coach (MacroFactor), and Apple Fitness data, with one feature none of them have: a unified training-load model across strength and endurance.

Not a product. One user, no multi-tenancy, no signup flow, no public deployment hardening beyond basic auth. Optimize for correctness of the models and speed of iteration, not scale. However, in the future, could think about turning this into a product.

Current phase: framework scaffold, then basic weight/food logging, then TDEE v0 (see Roadmap).

## Owner and working style

The owner is an experienced systems programmer (Rust, C, Python/NumPy, some Flask/HTML/JS/Bootstrap). Do not explain language basics. Web/full-stack experience is limited to Flask with server-rendered templates, so prefer server-rendered patterns and explain nontrivial web-architecture choices briefly when first introduced.

Work is split into two tiers.

**Tier A: move fast, decide autonomously.** Routes, handlers, templates, CSS, CRUD, migrations that only add things, config, tooling, tests for Tier A code. Do not ask for confirmation on routine choices; make a reasonable decision and mention it in the summary.

**Tier B: review gate. Stop and get approval before implementing.** Anything in `crates/core/src/analytics/`, unit conversions, energy/macro arithmetic, the data-model decisions listed under Invariants, ingestion deduplication logic, and any migration that alters or drops existing data. Process:
1. Write or update a spec in `docs/models/<name>.md`: the model, its derivation, assumptions, known biases, parameters with defaults, and the test plan.
2. Stop and ask for review. Do not write implementation code for the model until the owner approves the spec.
3. Implement exactly the approved spec. If implementation reveals a problem with the spec, stop and raise it rather than silently deviating.
4. Mark the spec status `APPROVED` / `IMPLEMENTED` at the top of the doc.

When in doubt about which tier something is, treat it as Tier B.

Make sure to commit work to the github in chunks, to serve as a sort of version control ensuring against unintentional Claude edits so I can roll back on them.

## Stack

| Layer | Choice | Notes |
|---|---|---|
| Language | Rust (stable), Cargo workspace | |
| HTTP | axum + tokio + tower-http | tower-http for static files, compression, tracing |
| DB | SQLite via sqlx | WAL mode. Compile-time checked queries (`query!`/`query_as!`) with offline data committed in `.sqlx/` |
| Migrations | sqlx-cli | Reversible migrations (`-r`) |
| Templates | askama | Compile-time checked templates |
| Interactivity | htmx | Server returns HTML partials. No SPA, no JS build step |
| Styling | Bootstrap 5 | Vendored into `static/vendor/`, not from a CDN |
| Charts | uPlot (time series), Chart.js if needed | Vendored. Data served as JSON from `/api/...` endpoints |
| Time | chrono + chrono-tz | |
| Auth | argon2 password hash + tower-sessions cookie; static bearer token for ingestion endpoint | Single user |
| Errors | thiserror in `core`, an `AppError` type in `server` implementing `IntoResponse` | |
| Logging | tracing + tracing-subscriber | |
| Testing | cargo test, proptest for analytics, approx for float comparisons | |
| Prototyping | Optional Jupyter notebooks in `notebooks/` | For exploring models on exported CSV before writing the Rust spec. Never imported by the app |

Escape hatch: if a specific UI surface (for example, a live in-gym workout logger with rest timers and offline tolerance) outgrows htmx, add Alpine.js or a small isolated JS module for that page only. Do not introduce a frontend framework or bundler without asking.

## Repository layout

```
.
├── CLAUDE.md
├── Cargo.toml                 # workspace
├── .env.example
├── crates/
│   ├── core/                  # pure domain + analytics. NO I/O, NO async, NO sqlx, NO axum
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── units.rs       # newtypes: Kg, Kcal, Grams, Meters, Seconds; conversions
│   │       ├── domain/        # WeighIn, FoodItem, Recipe, FoodLogEntry, DailyIntake, Workout, ...
│   │       └── analytics/     # tdee.rs, trend.rs, (later) load.rs. Tier B
│   └── server/                # axum app, db access, templates, ingestion
│       ├── src/
│       │   ├── main.rs
│       │   ├── config.rs
│       │   ├── error.rs
│       │   ├── auth.rs
│       │   ├── db/            # repository functions, one module per aggregate
│       │   ├── routes/        # one module per feature: weight, food, recipes, dashboard, api, ingest
│       │   └── ingest/        # Health Auto Export parsers (later)
│       ├── templates/         # askama: base.html, pages/, partials/
│       └── migrations/
├── static/
│   ├── css/app.css
│   ├── js/app.js              # minimal glue only
│   └── vendor/                # bootstrap, htmx, uplot
├── docs/
│   └── models/                # Tier B specs: tdee-v0.md, load-v0.md, ...
├── notebooks/                 # optional prototyping, not part of the build
└── data/                      # SQLite db lives here; gitignored
```

The `core`/`server` split is load-bearing: every number shown to the user that is derived (trend weight, TDEE, macro totals, training load) is computed in `core` by a pure function that is unit-tested without a database. `server` fetches rows, maps them to `core` types, calls `core`, and renders.

## Invariants (Tier B to change)

- **Canonical units in storage and in `core`:** mass kg, energy kcal, macros g, distance m, duration s. Conversion to lb / mi / etc. happens only at the presentation layer via `core::units`. The owner's display preference is lb; make display units a config setting.
- **Time:** instants stored as UTC (RFC 3339 text). Every daily-aggregated record (weigh-ins, food log, daily steps) also stores an explicit `local_date` computed from the configured user time zone at write time. Day-level analytics key on `local_date`, never on UTC dates.
- **History is immutable by snapshot:** a food log entry stores its computed kcal and macros at log time. Editing or deleting a food item or recipe never changes past log entries.
- **Ingestion is idempotent:** every externally sourced row carries `source` and `external_id` with a unique constraint on `(source, external_id)`. Re-importing the same export must be a no-op.
- **Derived values are not stored** unless caching is needed for performance, and then only in explicitly named cache tables that can be dropped and recomputed.

## Initial data model (M1 scope)

Sketch only; the migration is the source of truth.

- `weigh_in(id, measured_at_utc, local_date, weight_kg REAL, source TEXT, external_id TEXT NULL, note TEXT NULL)`. Unique on `(source, external_id)`. Multiple per day allowed; analytics uses the first of the day by default.
- `food_item(id, name, brand NULL, kcal_per_100g, protein_g_per_100g, carbs_g_per_100g, fat_g_per_100g, fiber_g_per_100g NULL, serving_name NULL, serving_g NULL, archived BOOL)`
- `recipe(id, name, yield_g NULL, servings NULL, archived BOOL)`, `recipe_ingredient(recipe_id, food_item_id, grams)`. (M3; create tables later.)
- `food_log(id, logged_at_utc, local_date, meal TEXT, food_item_id NULL, recipe_id NULL, grams NULL, kcal, protein_g, carbs_g, fat_g, fiber_g NULL, is_quick_add BOOL)`. Snapshot columns are authoritative.
- `day_meta(local_date PRIMARY KEY, intake_complete BOOL)`. Lets the owner mark a day as fully logged, which the TDEE estimator uses for coverage (a day with entries is not necessarily a complete day).

Future (do not create yet): `exercise`, `workout`, `strength_set`, `cardio_session`, `daily_activity` (steps, active energy), `hr_sample`.

## Commands

```bash
cargo run -p server                                     # dev server
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
sqlx migrate add -r <name> --source crates/server/migrations
sqlx migrate run --source crates/server/migrations
cargo sqlx prepare --workspace                          # after changing any query! macro; commit .sqlx/
```

Before reporting a task as done: `cargo fmt`, `cargo clippy` clean with `-D warnings`, `cargo test` passing, and `cargo sqlx prepare` rerun if queries changed.

## Configuration

Via environment (`.env` loaded with dotenvy; `.env.example` committed, `.env` gitignored):
`DATABASE_URL`, `BIND_ADDR`, `USER_TZ` (IANA name), `DISPLAY_MASS_UNIT` (`kg`|`lb`), `APP_PASSWORD_HASH` (argon2 PHC string), `SESSION_SECRET`, `INGEST_TOKEN`.

## Conventions

- Handlers stay thin: parse input, call `db`, call `core`, render. No business logic in handlers or templates.
- htmx endpoints that return partials live next to their full-page route and render templates from `templates/partials/`.
- Forms validate server-side; invalid input re-renders the form partial with errors (htmx swap), never a raw 4xx body.
- No `unwrap()`/`expect()` in `server` outside of startup. In `core`, only where an invariant is proven locally, with a comment.
- Floats: `f64` everywhere. Compare with `approx` in tests. Round only for display.
- Every `core::analytics` function gets unit tests on hand-computed cases plus property or synthetic-data tests described in its spec.
- Mobile-first layout. Most logging happens on a phone.
- Keep dependencies modest; ask before adding a crate that is large or pulls in a second async runtime, ORM, or templating system.

## Roadmap

- **M0 Scaffold (Tier A):** workspace, axum server with tracing, config, SQLite + migrations, base template with Bootstrap/htmx, login + session, health-check route, CI-equivalent `just`/script optional.
- **M1 Logging (Tier A, except invariants):** weigh-in CRUD, food items CRUD, food log with quick-add kcal/macros, daily summary page, mark-day-complete toggle.
- **M2 TDEE v0 (Tier B):** implement `docs/models/tdee-v0.md` once approved. Dashboard shows raw weights, trend line, current estimate with interval and data-quality flags.
- **M3 Recipe builder (Tier A):** recipes from food items, per-serving and per-gram macros, log a recipe by grams or servings.
- **M4 Training log (Tier A):** exercises, strength workouts with sets (reps, load, RPE), manual cardio sessions.
- **M5 Apple Health ingestion (Tier B for dedupe/mapping):** `POST /ingest/health-auto-export` accepting Health Auto Export JSON, bearer-token auth, idempotent upserts for weight, workouts, steps, HR.
- **M6 Unified training load (Tier B):** spec first in `docs/models/load-v0.md`.
- **Later:** state-space (Kalman) TDEE with adaptive expenditure, macro targets and adjustment logic, PWA/offline, native iOS HealthKit bridge.