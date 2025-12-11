# Rust Modern Backend Base

A production-ready Rust backend with **Clean Architecture**, JWT authentication, and OpenAPI documentation.

## Features

- ✅ Clean Architecture (Domain → Application → Infrastructure → API)
- ✅ JWT Authentication with Argon2 password hashing
- ✅ Role-Based Access Control (RBAC)
- ✅ Input Validation with `validator`
- ✅ Pagination support
- ✅ OpenAPI/Swagger documentation
- ✅ Request ID tracking & CORS
- ✅ Structured error handling

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Docker](https://www.docker.com/) & Docker Compose
- [sqlx-cli](https://github.com/launchbadge/sqlx) (`cargo install sqlx-cli --features postgres`)

### 1. Start Database

```bash
# Start PostgreSQL & Redis
docker-compose up -d
```

### 2. Setup Environment

```bash
# Copy env file
cp .env.example .env
```

### 3. Run Migrations

```bash
# Create database and run migrations
./scripts/init-db.sh

# Or manually:
sqlx migrate run
```

### 4. Start Server

```bash
cargo run -p api
```

### 5. Open Swagger UI

🌐 http://localhost:3000/swagger-ui/

## API Endpoints

| Method | Endpoint         | Auth | Description            |
| ------ | ---------------- | ---- | ---------------------- |
| POST   | `/auth/register` | ❌   | Register new user      |
| POST   | `/auth/login`    | ❌   | Login and get JWT      |
| GET    | `/users`         | ❌   | List users (paginated) |
| GET    | `/users/:id`     | ❌   | Get user by ID         |
| GET    | `/me`            | ✅   | Get current user       |
| GET    | `/health`        | ❌   | Health check           |

## Project Structure

```
rust_base/
├── crates/
│   ├── api/            # HTTP layer (Axum, handlers, middleware)
│   ├── application/    # Business logic & use cases
│   ├── domain/         # Entities, errors, repository traits
│   ├── infrastructure/ # DB repositories, auth implementations
│   └── shared/         # Configuration
├── migrations/         # SQL migrations
├── scripts/            # Helper scripts
├── docker-compose.yml  # PostgreSQL & Redis
└── .env.example        # Environment template
```

## Environment Variables

| Variable               | Default                  | Description                  |
| ---------------------- | ------------------------ | ---------------------------- |
| `DATABASE_URL`         | -                        | PostgreSQL connection string |
| `REDIS_URL`            | `redis://localhost:6379` | Redis connection string      |
| `JWT_SECRET`           | `super-secret-key...`    | JWT signing secret           |
| `JWT_EXPIRATION_HOURS` | `24`                     | Token expiration time        |
| `RUST_LOG`             | `info`                   | Log level                    |

## Tech Stack

- **Web Framework**: [Axum](https://github.com/tokio-rs/axum)
- **Database**: PostgreSQL + [SQLx](https://github.com/launchbadge/sqlx)
- **Cache**: Redis (ready for integration)
- **Auth**: JWT + Argon2
- **Docs**: [utoipa](https://github.com/juhaku/utoipa) (Swagger UI)
- **Runtime**: [Tokio](https://tokio.rs/)

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Build release
cargo build --release
```

## License

MIT
