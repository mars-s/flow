# Turso as Flow's embedded local store

Research note for wiring Turso in as Flow's local database. Every claim below
is cited to a primary source: the crate's page on crates.io, its source on
[github.com/tursodatabase/turso](https://github.com/tursodatabase/turso), or
[docs.turso.tech](https://docs.turso.tech). No blog posts, no secondary
write-ups. Checked 2026-08-18.

## 1. Crate name

Add **`turso`**, not `libsql`. `libsql` still exists and is maintained, but
its own README says where new work goes:

> "If you're starting a new project, you probably want to look into
> [Turso](https://github.com/tursodatabase/turso). libSQL is actively
> maintained, but new features are being developed in Turso."
> — [github.com/tursodatabase/libsql](https://github.com/tursodatabase/libsql)

`turso` is not a fork of `libsql` — it's a ground-up SQL engine written in
Rust, compatible with SQLite at the file-format and SQL-dialect level:

> "Turso Database is a project to build the next evolution of SQLite in
> Rust" ... contrasted with libSQL, which is "a fork rather than a rewrite."
> — [github.com/tursodatabase/turso README](https://github.com/tursodatabase/turso)

Current published versions (crates.io API, `max_stable_version`):

| Crate | Version | Description |
| --- | --- | --- |
| `turso` | 0.7.2 | "Turso Rust API" |
| `libsql` | 0.9.30 | "The libSQL database library" |

Sources: [crates.io/api/v1/crates/turso](https://crates.io/api/v1/crates/turso),
[crates.io/api/v1/crates/libsql](https://crates.io/api/v1/crates/libsql).

Project status is pre-1.0 but used in production:

> "Yes — Turso powers production applications today at multiple
> organizations, including Turso Cloud, the Kin AI assistant, and Spice.ai."
> ... "That said, we have not yet reached 1.0. The project is under active
> development, and some features are explicitly marked experimental."
> — [github.com/tursodatabase/turso README](https://github.com/tursodatabase/turso), FAQ

## 2. Opening a database and running a query

API surface confirmed against the real example shipped in the repo
([bindings/rust/examples/example.rs](https://github.com/tursodatabase/turso/blob/main/bindings/rust/examples/example.rs))
and the quickstart at
[docs.turso.tech/sdk/rust/quickstart](https://docs.turso.tech/sdk/rust/quickstart):

```rust
use turso::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Builder::new_local("app.db").build().await?;
    let conn = db.connect()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        )",
        (),
    ).await?;

    conn.execute("INSERT INTO users (name) VALUES (?1)", ("Alice",)).await?;

    let mut rows = conn.query("SELECT * FROM users", ()).await?;
    while let Some(row) = rows.next().await? {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        println!("User: {id} {name}");
    }

    Ok(())
}
```

Shape of the API, all `async`/`.await`:

- `Builder::new_local(path)` — `path` is a file path, or `":memory:"` for an
  in-memory database. `.build().await` returns a `Database`.
- `db.connect()` — returns a `Connection` **synchronously** in the local (no
  `sync` feature) path (`example.rs`). With the `sync` feature enabled, the
  shipped `sync_example.rs` calls `db.connect().await?` instead — connect
  becomes async once sync is wired in. Verify which shape applies before
  writing calling code.
- `conn.execute(sql, params)` and `conn.query(sql, params)` — run SQL
  directly; params are a tuple or array of bindable values.
- `conn.prepare(sql).await` — returns a `Statement`; `stmt.execute(params)`
  or `stmt.query(params)` for prepared statements.
- `rows.next().await?` — `Rows` is a streaming async iterator, one row at a
  time — there is no `.collect()`-style eager materialization in the
  examples.
- `row.get::<T>(index)` — typed extraction; `row.get_value(index)` — an
  untyped `Value` enum, used in `example.rs`'s pragma callback and the sync
  example.
- `conn.pragma_query(name, callback)` — pragma access, seen in `example.rs`.

## 3. Async runtime: tokio, not runtime-agnostic

The crate is not runtime-agnostic. Every example uses `#[tokio::main]`, and
`tokio` is a real dependency of the `turso` crate itself:

> `tokio` (with `"full"` features) is included as optional [in
> `bindings/rust/Cargo.toml`], required when the `sync` feature is enabled.
> — [github.com/tursodatabase/turso, `bindings/rust/Cargo.toml`](https://github.com/tursodatabase/turso/blob/main/bindings/rust/Cargo.toml)

The quickstart's own install line pulls tokio in directly:

```bash
cargo add turso tokio --features tokio/full
```

**Flow's current async story, checked against the actual codebase, not
assumed:**

```
$ grep -rn "tokio\|smol\|async-std" Cargo.toml src
Cargo.toml:43:smol = "2"
Cargo.toml:48:tokio = { version = "1", features = ["rt"] }
src/analytics.rs:267:    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
```

Flow already depends on **both** `smol` (GPUI's own executor,
`cx.background_executor()`) and a minimal `tokio` (`features = ["rt"]`).
`src/analytics.rs` builds a private `tokio::runtime::Builder::new_current_thread()`
on a dedicated background OS thread purely to drive the analytics HTTP
client — it never touches GPUI's executor. That's the existing precedent for
running tokio isolated from `smol`/GPUI: Turso would need the same treatment
(its own tokio runtime on a background thread, or upgrading the `tokio`
feature set from `["rt"]` to whatever Turso's own async internals need —
`rt-multi-thread`, `time`, `net` for the `sync` feature's HTTP client). This
does not solve the integration, just flags where it collides with Flow's
"no blocking work on the render thread" rule in `CLAUDE.md`.

## 4. Migrations

No built-in migration framework was found in any primary source — no
`turso migrate`, no `turso::migration` module in the docs or the examples.
It is raw SQL execution, same as `rusqlite`: `CREATE TABLE IF NOT EXISTS`
via `conn.execute(..)`, exactly as in every example above. If Flow wants
versioned migrations it will need to write that layer itself (a
`schema_version` pragma or table plus an ordered list of SQL strings is the
usual `rusqlite`-style pattern) — nothing in the `turso` crate does this for
you.

## 5. Turso Sync

**Status:** the `sync` feature exists in the `turso` crate today —
confirmed from the crate's own `Cargo.toml` (a `sync` feature flag pulling
in `hyper`, `hyper-rustls`, `hyper-util`, full `tokio`) and a real,
maintained example
([`bindings/rust/examples/sync_example.rs`](https://github.com/tursodatabase/turso/blob/main/bindings/rust/examples/sync_example.rs)).
But **it is not yet documented on docs.turso.tech for Rust**: the official
Sync usage page only shows TypeScript (`@tursodatabase/sync`), Python
(`turso.sync`), and Go (`turso.tech/database/tursogo`) —

> Rust is not mentioned anywhere in
> [docs.turso.tech/sync/usage](https://docs.turso.tech/sync/usage).

No explicit beta/GA label was found anywhere for Sync in general (Rust or
otherwise) — the closest signal is the repo-wide "we have not yet reached
1.0 ... some features are explicitly marked experimental" line quoted in
§1. Treat Sync as pre-1.0/unstable until Turso publishes an explicit
stability statement.

**Minimum crate version:** unconfirmed. The `sync` feature is present in
the `main` branch's `bindings/rust/Cargo.toml`; it was **not** independently
verified against the published `0.7.2` tag specifically (crates.io does not
expose per-version Cargo.toml diffs through the API endpoints checked here).
Pin and inspect the actual `0.7.2` source (or whatever version is added)
before relying on `sync` being present.

**Rust API**, verbatim from `sync_example.rs`:

```rust
use turso::sync::{Builder, RemoteEncryptionCipher};
use turso::Error;

let mut builder = Builder::new_remote(":memory:").with_remote_url(&remote_url);

if let Some(token) = auth_token {
    builder = builder.with_auth_token(token);
}

let db = builder.build().await?;
let conn = db.connect().await?;   // async here, unlike the plain local path

conn.execute("CREATE TABLE IF NOT EXISTS t (x TEXT)", ()).await?;
// ... normal query/execute calls ...
db.push().await?;                  // push local writes to Turso Cloud
// db.pull().await? also exists (docs.turso.tech quickstart, non-Rust pages)
let stats = db.stats().await?;     // network_received_bytes, network_sent_bytes, main_wal_size, ...
```

This example takes `Builder::new_remote(path).with_remote_url(url)` — the
local-file-plus-cloud-sync path (`Builder::new_local(path)` upgraded with
sync config) shown on the non-Rust docs pages was not found reproduced
verbatim for Rust anywhere in the primary sources checked; only
`new_remote` was seen in the actual Rust example. Confirm the exact local+
sync builder shape against the crate's own generated rustdoc
(`docs.rs/turso`) before shipping, since the general quickstart's prose
("local + cloud sync" recommended path, add `--features sync`) does not
match 1:1 with what `sync_example.rs` does.

Not needed for Flow's current phase — noted here only because the team
flagged it as a later phase.

## 6. Gotchas for a native desktop app

**Pure Rust, no libsqlite3 FFI — confirmed, not assumed.** The official
Rust connect-guide draws this out explicitly when comparing the three
Rust-facing crates:

> the `turso` and `turso_serverless` crates do **not require a C compiler**;
> the `libsql` crate **requires a C compiler** for its `core`,
> `replication`, and `encryption` features.
> — [docs.turso.tech/connect/rust](https://docs.turso.tech/connect/rust)

Combined with the repo's own framing — "An in-process SQL database written
in Rust, compatible with SQLite" with no FFI bindings to `libsqlite3`
mentioned anywhere in the README — `turso` is the pure-Rust option; `libsql`
(the older crate, see §1) is the one that still shells out to C. That's a
real advantage for Flow's bundling story: no `libsqlite3.dylib`/`.so` to
vendor or link against, no cross-compilation C toolchain concerns.

**Platform support.** The README states cross-platform coverage directly:

> "Cross-platform support for Linux, macOS, Windows and browsers (through
> WebAssembly)"
> — [github.com/tursodatabase/turso README](https://github.com/tursodatabase/turso)

Flow only targets macOS and Linux today, both inside Turso's stated support
matrix; Windows support exists but is moot for Flow.

**Default build features.** The crate's `Cargo.toml` enables `mimalloc` and
`fts` (full-text search) by default; `mimalloc` is skipped on wasm targets.
Neither was flagged as a build hazard in any source checked, but they are
extra compiled code Flow gets even if unused — worth a `default-features =
false` pass later if binary size or build time becomes a concern.

**Thread-safety / concurrency — not documented anywhere checked.** No
primary source (README, docs.turso.tech reference page, or crate docs)
states `Connection`'s `Send`/`Sync` bounds or gives guidance on sharing one
connection across tasks or threads. The repo does ship a
`concurrent_writes.rs` example titled "MVCC mode: 16 concurrent writers
using BEGIN CONCURRENT," which implies the engine supports concurrent
writers at the SQL/transaction level, but that is a different question from
whether a single `Connection`/`Statement` value is safe to hand across
threads. **Flag for integration, not solved here:** GPUI drives its own
async executor via `cx.background_executor()` (backed by `smol`), which is
a different runtime from the `tokio` that `turso` requires — Flow will need
to decide whether Turso calls run on a dedicated tokio runtime (the
`analytics.rs` pattern, §3) and results are marshaled back via
`cx.background_executor().spawn` + `cx.notify()`, or whether some other
bridging is used. Do not call Turso's `.await` points directly from a GPUI
render path.

## Cargo.toml additions

```toml
turso = "0.7.2"
tokio = { version = "1", features = ["rt"] }   # already present; may need more (see §3)
```

Add `features = ["sync"]` to the `turso` dependency only when the later
sync phase actually starts, and re-verify §5's open items (minimum version,
local+sync builder shape, stability) at that time — don't carry it in now
speculatively.

## Minimal working example (local file, one query)

Verified shape, combining `example.rs` and the quickstart (§2):

```rust
use turso::Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db = Builder::new_local("flow.db").build().await?;
    let conn = db.connect()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
        (),
    )
    .await?;

    let mut rows = conn.query("SELECT id, title FROM tasks", ()).await?;
    while let Some(row) = rows.next().await? {
        let id: i64 = row.get(0)?;
        let title: String = row.get(1)?;
        println!("{id}: {title}");
    }

    Ok(())
}
```

This is a plain `#[tokio::main]` binary for illustration only — inside Flow
it must run on a background tokio runtime reached from
`cx.background_executor()`, per §6, not on `main`.

## Open questions / verify before shipping

- **`db.connect()` sync vs async.** `example.rs` calls it without `.await`;
  `sync_example.rs` calls it with `.await`. Unclear whether this is a
  feature-gated signature change (`sync` feature makes `connect` async) or
  a version drift between the two examples. Check `docs.rs/turso/0.7.2`'s
  actual generated signature before writing calling code.
- **Exact 0.7.2 feature set.** The `Cargo.toml` inspected was read off the
  `main` branch, not the `0.7.2` tag. Confirm the `sync` feature (and its
  exact dependency set) exists in whatever version ends up pinned.
- **Turso Sync stability/versioning for Rust specifically.** docs.turso.tech
  documents Sync for TypeScript/Python/Go only; no Rust-specific sync
  guide, no explicit beta/GA label was found. Not blocking — sync is a
  later phase — but don't assume parity with the documented SDKs.
- **Local-file-plus-cloud-sync builder shape in Rust.** Only
  `Builder::new_remote(":memory:").with_remote_url(...)` was seen in the
  actual Rust sync example. The "local file synced to cloud" pattern
  described in prose on the general quickstart page was not confirmed
  against real Rust code — re-derive it from `docs.rs/turso`'s
  `turso::sync` module docs when the sync phase starts.
- **`Send`/`Sync` bounds on `Connection`/`Statement`.** Not stated anywhere
  in primary sources. Needs either a docs.rs check of the actual trait
  bounds or a source read of `bindings/rust/src` before deciding how to
  bridge Turso's tokio calls into GPUI's `smol`-backed executor.
- **Tokio feature set beyond `["rt"]`.** Flow's current `tokio` dependency
  only enables `"rt"`. Turso's own `Cargo.toml` pulls `tokio` with
  `"full"` when `sync` is on; the non-sync path's actual required tokio
  features (just `rt`? `time`? `net` for anything?) were not independently
  enumerated here — check `docs.rs/turso`'s Cargo.toml dependency listing
  for the pinned version before assuming `["rt"]` is sufficient.
- **Migration tooling.** Confirmed absent from every primary source
  checked, but worth one more look at `docs.rs/turso` module list (a
  `migration` or `schema` module could exist without being in the
  examples) before committing to hand-rolled migrations.
