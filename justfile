run-dev:
    @echo "Running development server..."
    cargo run -p observer-client

test:
    @echo "Running tests..."
    cargo nextest run

dev-setup:
    @echo "Setting up development environment..."
    cargo install --locked cargo-nextest