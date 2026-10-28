# AGENTS.md — Rust Project Best Practices

This document defines the conventions, best practices, and mandatory requirements for development in the **done-mock** project (written in Rust).

---

## 📋 Mandatory Checklist Before Submitting

Before any commit or PR, **all** the following items must pass:

```bash
# 1. Formatting — must pass without changes
cargo fmt --check

# 2. Lints — warnings treated as errors
cargo clippy -- -D warnings

# 3. Tests — all tests must pass
cargo test
```

These commands are executed automatically by the `pre-commit` hook in `.git-hooks/pre-commit`.

---

## 🌐 Language Requirements

All comments, test names, test descriptions, and documentation (rustdoc) must be written in **English**.

This ensures consistency across the codebase and makes the project accessible to international contributors.

---

## 🏗️ Project Structure

```
src/
├── main.rs          # Entry point, server initialization
├── lib.rs           # Public module declarations
├── admin.rs         # Admin API (mock CRUD)
├── config.rs        # Application configuration (AppConfig)
├── error.rs         # Error handling and custom error types
├── live_feed.rs     # Real-time feed (SSE/broadcast)
├── matching.rs      # Request matching with MockIndex
├── mock_server.rs   # Axum router and MockServerState
├── model.rs         # Data models (Mock, MockResponse, etc.)
├── responder.rs     # Response logic (SequenceState, dispatch)
├── script.rs        # JavaScript engine (QuickJS/rquickjs) [feature: script]
├── ui.rs            # Static file server for the UI [feature: ui]
└── store/
    ├── mod.rs       # MockStore trait
    ├── memory.rs    # In-memory implementation
    └── postgres.rs  # PostgreSQL implementation (sqlx) [feature: postgres]

ui/
├── index.html       # Main HTML entry point for the UI
├── app.js           # Frontend JavaScript logic
└── styles.css       # UI styles and theming
```

### Architectural Patterns

- **Trait-based abstractions**: `MockStore` is an async trait with implementations for in-memory and PostgreSQL.
- **Shared state via `Arc`**: `MockServerState` groups all shared components.
- **Cache + Index**: Mocks are cached in `Arc<RwLock<Vec<Mock>>>` and indexed in `MockIndex` for efficient lookup.
- **Builder pattern**: The `Mock` struct uses `MockBuilder` for fluent construction.

---

## 🎨 UI (Optional)

The project includes a static admin UI served at the root path `/` by the `ui.rs` module. This module is optional and controlled by the `ui` Cargo feature (enabled by default). When disabled, the UI routes are removed and static assets are not embedded in the binary.

### Structure

```
ui/
├── index.html       # Main HTML entry point with embedded form and table structure
├── app.js           # Frontend JavaScript logic (mock CRUD, live feed, event handling)
└── styles.css       # UI styles, theming, and responsive layout
```

### Key Features

- **Mock Management**: Create, edit, and delete mocks via a form-based interface.
- **Live Feed**: Real-time request/response monitoring using Server-Sent Events (SSE).
- **Mock Types Support**: Static, Conditional, Sequence, Script, and REST CRUD mock types.
- **Tab Navigation**: Switch between Mocks list and Live Feed views.

### Conventions

- **No build step**: The UI is pure HTML/CSS/JS with no framework or build tooling.
- **Static assets**: Served directly from the `ui/` directory via Axum's static file server.
- **API Integration**: All data operations use the admin API (`/__admin/mocks`, `/__admin/live-feed`).
- **Responsive design**: CSS uses flexbox and grid for responsive layouts.
- **Event-driven**: JavaScript uses event delegation for dynamic table rows and form handling.

### Serving

The `ui.rs` module configures Axum to serve static files from the `ui/` directory. The frontend makes API calls to the admin endpoints to manage mocks and subscribe to the live feed via SSE.

---

## 📝 Code Conventions

### Formatting

- Run `cargo fmt` before each commit.
- The project uses the default `rustfmt` settings — **do not** create `rustfmt.toml` unless necessary.
- Indentation with 4 spaces.
- Maximum line length: 100 characters (rustfmt default).

### Clippy Lints

- Run `cargo clippy -- -D warnings` — **all warnings are errors**.
- If lint must be genuinely suppressed, use `#[allow(clippy::lint_name, reason = "...")]` with a justification.
- Existing example in the project:

```rust
#[allow(
    clippy::double_must_use,
    reason = "async_trait macro duplicates must_use in async methods"
)]
```

### Naming Conventions

| Element | Convention | Example |
|----------|-----------|---------|
| Modules | snake_case | `mock_server`, `live_feed` |
| Structs/Enums | PascalCase | `MockServerState`, `MockType` |
| Functions/Methods | snake_case | `find_best_match`, `next_response` |
| Constants | SCREAMING_SNAKE_CASE | `DEFAULT_PORT` |
| Private fields | snake_case with `_` prefix (if applicable) | `mock_cache` |
| Acronyms | lowercase in snake_case | `http_client`, `json_body` |

### Imports

- Order imports: std → external crates → internal modules.
- Use `use crate::module::Type` for internal references.
- Group imports from the same crate when possible.

```rust
// std first
use std::collections::HashMap;
use std::sync::Arc;

// external crates
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

// internal modules
use crate::model::Mock;
use crate::store::MockStore;
```

---

## 🧪 Tests

### Structure

- **Integration tests**: `tests/integration_tests.rs` — end-to-end tests with a real server.
- **Unit tests**: `#[cfg(test)]` modules inside source files.
- Use `spawn_test_server()` helper to create servers on random ports.

### Best Practices

- Name tests descriptively: `test_admin_api_and_static_mock_lifecycle`.
- Each test must be independent and not depend on a global state.
- Use `#[tokio::test]` for async tests.
- Cover both positive and negative cases (errors, invalid input).
- For tests involving a database, use `InMemoryMockStore` by default.

### Test Example

```rust
#[tokio::test]
async fn test_create_and_verify_static_mock() {
    let base_url = spawn_test_server().await;
    let client = reqwest::Client::new();

    // Arrange
    let mock = Mock::builder()
        .name("Test Mock")
        .method("GET")
        .path("/test")
        .build();

    // Act
    let res = client
        .post(format!("{}/__admin/mocks", base_url))
        .json(&mock)
        .send()
        .await
        .unwrap();

    // Assert
    assert_eq!(res.status(), 201);
}
```

---

## 🔧 Rust Code Patterns

### Error Handling

- Use `thiserror` for custom error enums (`MockServerError`) in the `MockStore` trait and admin handlers.
- Use `anyhow::Result` only where a general-purpose error type is needed.
- Propagate errors with `?` — avoid `unwrap()` except in tests and initialization code.
- In Axum handlers, convert `MockServerError` variants to appropriate HTTP responses using `status_code()` and `error_type()`.

### Async Patterns

- Use `#[tokio::main]` at the entry point.
- Use `#[async_trait::async_trait]` for async traits.
- Prefer `Arc<tokio::sync::RwLock<T>>` for shared async state.
- Avoid `.await` inside `Fn` closures — use `async_closure` or restructure.

### Ownership and Borrowing

- Use `Arc<T>` for cross-thread sharing.
- Use `Rc<T>` only in single-threaded contexts (rare in this project).
- Prefer references (`&T`) over clones when possible.
- Use `Cow<str>` when a function may return either borrowed or owned data.

### Serde Patterns

- Use `#[serde(default)]` for optional fields with default values.
- Use `#[serde(skip_serializing_if = "Option::is_none")]` to omit null fields.
- Use `#[serde(rename_all = "camelCase")]` for JSON APIs.
- For UUIDs, use `#[serde(default = "default_uuid")]` with a function that generates a UUID.

### Logging

- Use `tracing` for structured logging.
- Levels: `info!` for important events, `debug!` for diagnostics, `error!` for failures.
- Configure via `RUST_LOG` environment variable.

```rust
use tracing::info;

info!(mock_id = %mock.id, "Mock created successfully");
```

---

## 📦 Dependencies and Features

### Main Dependencies

| Crate | Version | Optional | Usage |
|-------|---------|----------|-------|
| `tokio` | 1.38 | No | Async runtime (selective features) |
| `axum` | 0.8.9 | No | Web framework |
| `serde` | 1.0 | No | Serialization/deserialization |
| `sqlx` | 0.9.0 | Yes (`postgres` feature) | Database with type-safe queries |
| `rquickjs` | 0.14 | Yes (`script` feature) | JavaScript engine for scripts |
| `thiserror` | 2.0 | No | Custom error enum derivation |
| `tracing` | 0.1 | No | Structured logging |
| `uuid` | 1.8 | No | Unique identifiers |
| `chrono` | 0.4 | No | Dates and timestamps |

### Cargo Features

The project uses Cargo features to make optional components conditional at compile time:

| Feature | Default | Gates |
|---------|---------|-------|
| `postgres` | ✅ | `sqlx` dependency + `src/store/postgres.rs` |
| `script` | ✅ | `rquickjs` dependency + `src/script.rs` + script mock handler |
| `ui` | ✅ | `src/ui.rs` + UI routes and static assets |

All features are enabled by default. Key implications:

- **`--no-default-features`**: Produces a minimal binary with only the in-memory store, no script engine, and no UI.
- **`--features ui`**: Adds only the UI module and embedded static assets.
- **`--features script`**: Adds the QuickJS engine for Script mocks.
- **`--features postgres`**: Adds PostgreSQL support via `sqlx`.

The Docker image builds with all default features. A minimal image can be built by passing `--no-default-features` to the `cargo build` command in the Dockerfile.

### Dependency Management

- Keep versions up to date with `cargo update`.
- Add new dependencies only when justified.
- Prefer well-maintained and popular crates.
- Use selective features instead of `features = ["full"]`.

---

## 🐳 Docker and CI/CD

### Dockerfile

- The project uses Docker multi-stage build.
- The final image should be minimalist (distroless).
- Run `docker build -t done-mock .` locally to verify.

### GitHub Actions

- Workflow in `.github/workflows/rust.yml` runs two jobs:
  1. **verify** — `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
  2. **docker-build** — builds the Docker image to verify the musl cross-compilation pipeline
- Add new jobs only when necessary.

---

## 🔄 Git Hooks

### Pre-commit (`.git-hooks/pre-commit`)

Runs automatically before each commit:
- `cargo fmt --check` — validates formatting
- `cargo clippy -- -D warnings` — validates lints
- `cargo test` — runs all tests

### Commit Message (`.git-hooks/commit-msg`)

Validates commit messages with **Conventional Commits**:

```
<type>(<scope>): <description>
```

**Allowed types:**
- `feat`: new feature
- `fix`: bug fix
- `docs`: documentation
- `style`: formatting, semicolons, no code change
- `refactor`: refactoring without behavior change
- `test`: adding or fixing tests
- `chore`: general maintenance
- `perf`: performance improvement
- `build`: changes to the build system or dependencies
- `ci`: changes to CI files
- `revert`: revert previous commit
- `wip`: work in progress

**Rules:**
- Maximum message length: 200 characters
- Scope is optional

**Valid examples:**
```
feat(matching): add mock index by method and path
fix(store): handle partial update correctly in postgres
test(integration): add lifecycle test for conditional mocks
refactor(model): extract merge logic to Mock::apply_partial
```

---

## 📚 Documentation

### Rustdoc

- Document all public structs, enums, and methods with `///`.
- Include usage examples when applicable.
- Run `cargo doc --open` to verify the documentation.

```rust
/// Finds the best matching mock for a request.
///
/// Uses `MockIndex` for efficient lookup by method and path,
/// then applies matching rules to select the candidate.
///
/// # Examples
///
/// ```
/// let result = find_best_match(&index, &request);
/// assert!(result.is_match());
/// ```
pub fn find_best_match(index: &MockIndex, req: &MockRequest) -> MatchResult {
    // ...
}
```

### README and Docs

- Keep `README.md` and `AGENTS.md` updated with usage instructions.
- Detailed technical documentation in `docs/`.

---

## ⚡ Performance

### Best Practices

- **Avoid unnecessary clones**: Use references and `Arc<T>` for sharing.
- **Cache expensive results**: `MockServerState` already caches mocks.
- **Indexing**: Use `MockIndex` for O(1) lookup by method/path.
- **Connection pool**: PostgreSQL pool configurable via `POOL_MAX_CONNECTIONS`.
- **Atomic operations**: Use `AtomicUsize` for counters instead of locks.

### Anti-Patterns

- ❌ `store.list()` on every request — use the cache.
- ❌ Creating a QuickJS `AsyncRuntime` per script — use the shared engine.
- ❌ Linear O(n) search without indexing — use `MockIndex`.
- ❌ `unwrap()` in production code — use `?` or `expect()` with a message.
- ❌ `println!()` — use `tracing` macros.

---

## 🔒 Security

- Never commit secrets or tokens.
- Use `.env` for environment variables (already in `.gitignore`).
- Validate user input in admin APIs.
- Consider rate limiting on `/__admin` routes.

---

## 🧹 Code Review Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] Commit message follows Conventional Commits
- [ ] New functions have rustdoc documentation
- [ ] Errors are handled with `?` or `expect()` with a message
- [ ] No `unwrap()` in production code
- [ ] Logs use `tracing` instead of `println!`
- [ ] Tests cover the main cases
- [ ] New dependencies are justified

---

## 🐳 Docker & CI/CD

### Dockerfile

The project uses a **Docker multi-stage build** that produces a **statically linked binary** via the `x86_64-unknown-linux-musl` target:

- **Builder stage**: `ghcr.io/rust-cross/rust-musl-cross:x86_64-musl` — pre-configured image with musl toolchain, works on any host architecture (including M1/M2 Macs via QEMU). The target is set as the default, so `cargo build --release` automatically compiles for musl.
- **Build command**: `cargo build --release --locked` produces a fully static binary with no dynamic library dependencies.
- **Binary stripping**: `musl-strip` reduces the binary size after building.
- **Production stage**: `gcr.io/distroless/static-debian13:nonroot` — the `static` variant is designed for statically linked binaries; it has a smaller attack surface and does not include the C standard library.
- **No health check**: The `distroless/static` image has no shell or utilities, so Docker-level health checks are not possible. Use orchestration tools (Kubernetes, docker-compose) for external health monitoring.

Build the image locally with:

```bash
docker build -t done-mock .
```

### docker-compose.yml

The `docker-compose.yml` file builds the image from the `Dockerfile` automatically — no manual image tag is required. It also includes a PostgreSQL service for the database backend.

```bash
docker compose up --build
```

### GitHub Actions

The CI workflow (`.github/workflows/rust.yml`) runs on every push and pull request to `main`:

1. **verify** job:
   - `cargo fmt --check` — validates formatting
   - `cargo clippy -- -D warnings` — validates lints
   - `cargo test` — runs all tests
2. **docker-build** job — builds the Docker image to verify the musl cross-compilation pipeline end-to-end

The verify job does not target the musl cross-compilation — it only checks the standard Rust toolchain. The docker-build job validates that the Dockerfile produces a working image.

---

## 📖 References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Axum Documentation](https://docs.rs/axum/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
