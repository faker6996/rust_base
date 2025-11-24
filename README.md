# Rust Modern Backend Base

This is a modern Rust backend codebase following **Clean Architecture** principles.

## Architecture

The project is organized as a Cargo Workspace with the following crates:

- **`crates/domain`**: Contains core business entities and repository interfaces (Ports). No external dependencies (mostly).
- **`crates/application`**: Contains business logic and use cases. Depends on `domain`.
- **`crates/infrastructure`**: Contains implementation of interfaces (Adapters), e.g., Database repositories using SQLx. Depends on `domain` and `application`.
- **`crates/api`**: The entry point of the application (Axum server). Wires everything together (Dependency Injection).
- **`crates/shared`**: Common utilities, configuration, and error types.

## Tech Stack

- **Web Framework**: [Axum](https://github.com/tokio-rs/axum)
- **Database**: PostgreSQL with [SQLx](https://github.com/launchbadge/sqlx)
- **Runtime**: [Tokio](https://tokio.rs/)
- **Configuration**: [config-rs](https://github.com/mehcode/config-rs)
- **Observability**: [tracing](https://github.com/tokio-rs/tracing)

## Getting Started

### Prerequisites

- Rust (latest stable)
- PostgreSQL
- `sqlx-cli` (optional, for migrations)

### Setup

1.  **Database Setup**:

    ```bash
    # Create a database
    createdb rust_base_db

    # Run migrations
    export DATABASE_URL=postgres://user:password@localhost/rust_base_db
    sqlx migrate run
    ```

2.  **Run the Server**:
    ```bash
    export DATABASE_URL=postgres://user:password@localhost/rust_base_db
    cargo run -p api
    ```

### Testing

```bash
cargo test
```

## Project Structure

```
.
├── Cargo.toml              # Workspace definition
├── migrations/             # SQLx migrations
├── crates/
│   ├── api/                # HTTP layer (Axum)
│   ├── application/        # Business logic
│   ├── domain/             # Entities & Interfaces
│   ├── infrastructure/     # DB & External adapters
│   └── shared/             # Shared utils
└── README.md
```
