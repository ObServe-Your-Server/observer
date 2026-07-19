run-dev:
    @echo "Running development server..."
    cargo run -p observer-client

test:
    @echo "Running tests..."
    cargo nextest run

dev-setup:
    @echo "Setting up development environment..."
    cargo install --locked cargo-nextest

migrate-generate name:
    @echo "Generating migration {{name}}..."
    cd bins/client && sea-orm-cli migrate generate {{name}}

migrate-up:
    @echo "Applying migrations..."
    cd bins/client && DATABASE_URL="sqlite://../../observer.db?mode=rwc" sea-orm-cli migrate up

entity-generate:
    @echo "Generating entities from database..."
    sea-orm-cli generate entity -u "sqlite://observer.db?mode=rwc" -o bins/client/src/entities

codegen: migrate-up entity-generate