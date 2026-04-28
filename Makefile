# Makefile

.PHONY: build fmt fmt-check lint test clean install setup release package publish docs help

BINARY  := $(shell grep '^name'    Cargo.toml | head -1 | sed 's/.*= "//' | sed 's/"//')
VERSION := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*= "//' | sed 's/"//')
TARGET  := x86_64-unknown-linux-musl
PREFIX  ?= /usr/local

## help - show available targets
help:
	@grep -E '^## [a-zA-Z_-]+ - ' Makefile | awk 'BEGIN {FS=" - "} {printf "  %-15s %s\n", substr($$1, 4), $$2}'

## build - compile a static musl release binary
build:
	cargo build --release --target $(TARGET)

## fmt - auto-format code with cargo fmt
fmt:
	cargo fmt

## fmt-check - check code formatting without modifying files
fmt-check:
	cargo fmt --check

## lint - check formatting and run clippy
lint:
	$(MAKE) fmt-check
	cargo clippy -- -D warnings

## test - run the test suite
test:
	cargo test

## clean - remove build artifacts
clean:
	cargo clean

## install - install the binary to $(PREFIX)/bin (default: /usr/local/bin)
install: build
	install -Dm755 target/$(TARGET)/release/$(BINARY) $(PREFIX)/bin/$(BINARY)

## setup - install all tools and dependencies required to work on this project
setup:
	@command -v rustup >/dev/null 2>&1 || curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	@. "$$HOME/.cargo/env" && rustup toolchain install stable && rustup target add $(TARGET) && rustup component add clippy rustfmt
	sudo apt-get install -y musl-tools

## release - tag the current version and push to trigger the release pipeline
release:
	git tag v$(VERSION)
	git push origin v$(VERSION)

## package - build .deb and AUR packages from the release binary
package:
	$(MAKE) build
	$(MAKE) build-deb
	$(MAKE) build-aur

## publish - publish the crate to crates.io
publish:
	cargo publish

## docs - open generated crate documentation in the browser
docs:
	cargo doc --no-deps --open

build-deb:
	@scripts/build-deb.sh $(BINARY) $(VERSION)

build-aur:
	@scripts/build-aur.sh $(BINARY) $(VERSION)

# ── Project-specific targets ──────────────────────────────────────────────────
# Add targets below that are unique to this project. They will appear in
# `make help` automatically if you use the `## target - description` convention.
# Examples: database migrations, code generation, deployment steps, dev server.
