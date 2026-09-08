# List all available recipes
default:
    just --list --unsorted

# Update flake.lock and Cargo dependencies
update:
    nix flake update && cargo update

# Run all tests
test:
    cargo test

# Build the project in debug mode
build:
    cargo build

# Remove build artifacts
clean:
    cargo clean

# Run the binary with optional arguments (e.g. `just run hello`)
run *args:
    cargo run -- {{ args }}

# Publish the crate to crates.io
publish:
    cargo publish
