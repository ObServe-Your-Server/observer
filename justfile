test:
    @echo "Running tests..."
    cargo nextest run

dev-setup:
    @echo "Setting up development environment..."
    cargo install --locked cargo-nextest