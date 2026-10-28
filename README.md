# done-mock

A customizable mock HTTP server built with Rust, Axum, and `rquickjs` (QuickJS). Inspired by [sMockin](https://github.com/matthewgallina/smockin).

> **Note:** This project was previously named **double-deno-mock** (when it used Deno). The v1 version with Deno is still available on the [`v1`](https://github.com/svbgabriel/done-mock/tree/v1) branch. The project has been rewritten in Rust and renamed to **done-mock**.

## Features

- **Static Mocks**: Fixed status, headers, and body.
- **Conditional Mocks**: Branching logic based on headers, query params, or JSON body fields.
- **Sequence Mocks**: Returns a sequence of responses across multiple calls (supports cycling or stopping at the end).
- **Script Mocks**: Write custom logic in JS to compute the response, executed in a sandboxed QuickJS runtime via `rquickjs`.
- **REST Mocks**: Stateful REST API simulation with persistent resource collections and CRUD support.
- **Management UI**: Simple web interface at `/ui` to manage mocks.
- **Persistence**: Mocks are stored in-memory (default) or PostgreSQL.
- **Live Feed SSE**: Real-time streaming of request history and events at `GET /__admin/livefeed`.

### Cargo Features

The project supports optional Cargo features that control which components are compiled:

| Feature | Default | Description |
|---------|---------|-------------|
| `postgres` | ✅ | PostgreSQL persistence via `sqlx`. Disabling removes the database dependency entirely. |
| `script` | ✅ | JavaScript engine via `rquickjs` for Script mocks. Disabling removes QuickJS from the binary. |
| `ui` | ✅ | Static admin UI served at `/ui`. Disabling removes embedded HTML/CSS/JS assets. |

All features are enabled by default. Build a minimal binary with:

```bash
cargo build --release --no-default-features
```

Or enable individual features:

```bash
cargo build --release --no-default-features --features ui
cargo build --release --no-default-features --features postgres,ui
```

## Getting Started

### Prerequisites

- Rust and Cargo installed.

### Run the server

```bash
cargo run
```

The server will start on port 8000 by default. Access the UI at `http://localhost:8000/ui`.

### Environment Variables

- `PORT`: Server port (default: 8000).
- `DB_DRIVER`: `postgres` or `memory` (default).
- `POSTGRES_URL`: PostgreSQL connection URL. Used only if `DB_DRIVER=postgres`.
- `SCRIPT_TIMEOUT_MS`: Script execution timeout in milliseconds (default: 2000).
- `POOL_MAX_CONNECTIONS`: Maximum number of PostgreSQL connections in the pool (default: 10).
- `MOCKS_DIR`: Path to a directory of `.json` mock files to load at startup. Each file should contain a single mock definition. Invalid files are skipped with a warning.

### Docker

Build and run with Docker Compose (includes PostgreSQL):

```bash
docker compose up --build
```

The server will be available at `http://localhost:8000`.

To run without PostgreSQL (in-memory mode):

```bash
docker build -t done-mock .
docker run -p 8000:8000 -e DB_DRIVER=memory done-mock
```

### Seeding Mocks at Startup

Load mocks from a directory of JSON files by setting the `MOCKS_DIR` environment variable:

```bash
MOCKS_DIR=./examples/mocks cargo run
```

With Docker, mount a directory containing mock JSON files:

```bash
docker run -p 8000:8000 -v ./examples/mocks:/app/mocks:ro -e MOCKS_DIR=/app/mocks done-mock
```

Example mock files are available in the `examples/mocks/` directory.

### Bulk Admin API

You can also load mocks programmatically after the server starts:

- **`POST /__admin/mocks/bulk`**: Accepts a JSON array of mock definitions and creates them all. Returns created mocks and any validation errors.
- **`POST /__admin/mocks/reset`**: Deletes all mocks and resets sequence counters. Useful for test isolation.

## Development

### Build and Run

- `cargo run`: Start the server with all features enabled.
- `cargo run --no-default-features`: Start with only the in-memory store (no PostgreSQL, no scripts, no UI).
- `cargo test`: Run all tests (with default features).

### Test

```bash
cargo test
```

## How to define mocks

Mocks match incoming requests by **Method** and **Path Pattern** (supports URL pattern syntax like `/users/:id`). When multiple mocks match, the one with the highest **Priority** is chosen.

### Static Mock Example

Always returns the same response (fixed status, headers, and body):

```json
{
  "method": "GET",
  "path": "/api-test",
  "type": "static",
  "response": {
    "status": 200,
    "headers": { "Content-Type": "application/json" },
    "body": "{\"message\":\"ok\"}"
  }
}
```

### Conditional Mock Example

Evaluates a list of conditions in order and returns the response of the first match. If none match, returns `elseResponse`:

```json
{
  "method": "GET",
  "path": "/cond-example",
  "type": "conditional",
  "conditions": [
    {
      "source": "header",
      "key": "x-api-key",
      "op": "equals",
      "value": "secret",
      "response": { "status": 200, "body": "authorized" }
    },
    {
      "source": "query",
      "key": "debug",
      "op": "exists",
      "response": { "status": 200, "body": "debug mode" }
    }
  ],
  "elseResponse": { "status": 401, "body": "unauthorized" }
}
```

### Sequence Mock Example

Returns responses in sequence on each call. With `sequenceMode: "cycle"`, the sequence restarts when it reaches the end; with `"stopAtEnd"`, it remains on the last response:

```json
{
  "method": "GET",
  "path": "/seq-example",
  "type": "sequence",
  "sequenceMode": "cycle",
  "sequence": [
    { "status": 200, "body": "first call" },
    { "status": 200, "body": "second call" },
    { "status": 500, "body": "third call fails" }
  ]
}
```

### Script Mock Example

User scripts run in a sandboxed QuickJS runtime and receive a global `context` object representing the incoming request:

- `context.method`: Request HTTP method (e.g. `'GET'`, `'POST'`).
- `context.path`: Request path (e.g. `'/users/42'`).
- `context.pathParams`: Object with named path parameters from route matching (e.g. `{ id: '42' }` for `/users/:id`).
- `context.query`: Object with URL query parameters (e.g. `context.query.filter`).
- `context.headers`: Object with request headers in lowercase (e.g. `context.headers['content-type']`).
- `context.body`: Parsed JSON request body (if `Content-Type` is `application/json`), or `null`.

#### Example

For a route defined with path `/users/:id`:

```javascript
const userId = context.pathParams.id
const format = context.query.format || 'json'

return {
  status: 200,
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    id: userId,
    format: format,
    method: context.method,
    clientHeader: context.headers['x-client-id']
  })
}
```

### REST Mock

The `rest` type simulates a full RESTful API for a collection of resources. It maintains a persistent state (a JSON array) in the database.

To use it effectively, define a path pattern that includes an optional `:id` parameter, such as `/api/users/:id?` or create two mocks if needed, but typically a single pattern like `/api/items/:id?` works if the engine matches it.
*Note: The current implementation decides between collection and item operations based on whether the `:id` path parameter is present in the matched request.*

#### Supported Operations

| Path          | Method   | Action                                                           |
|:--------------|:---------|:-----------------------------------------------------------------|
| Collection    | `GET`    | Returns the full array of resources.                             |
| Collection    | `POST`   | Appends a new item. Generates a UUID if the ID field is missing. |
| Collection    | `PUT`    | Replaces the entire collection with the provided array.          |
| Collection    | `DELETE` | Clears the collection.                                           |
| Item (`/:id`) | `GET`    | Returns the resource with the matching ID.                       |
| Item (`/:id`) | `PUT`    | Replaces the matching resource.                                  |
| Item (`/:id`) | `PATCH`  | Performs a shallow merge on the matching resource.               |
| Item (`/:id`) | `DELETE` | Removes the resource from the collection.                        |

#### Configuration

- **Initial State**: A JSON array of objects to seed the collection.
- **ID Field**: The property name used as the unique identifier (default: `id`).
- **Reset**: You can reset the state back to the Initial State via the Management UI.

#### Example

```json
{
  "method": "*",
  "path": "/api/items/:id?",
  "type": "rest",
  "restIdField": "id",
  "restInitialState": [
    { "id": 1, "name": "First item" },
    { "id": 2, "name": "Second item" }
  ]
}
```

## Admin API (`/__admin`)

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/__admin/mocks` | List all registered mocks |
| `GET` | `/__admin/mocks/:id` | Get mock by ID |
| `POST` | `/__admin/mocks` | Create a new mock with validations |
| `PUT` | `/__admin/mocks/:id` | Update mock and reset sequence |
| `DELETE` | `/__admin/mocks/:id` | Delete mock and reset sequence |
| `POST` | `/__admin/mocks/:id/reset` | Reset sequence index and restore REST mock initial state |
| `GET` | `/__admin/livefeed` | SSE streaming of history and real-time events |
| `GET` | `/__admin/health` | Application health check |
