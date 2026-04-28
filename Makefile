# Makefile — targets implemented by project type initialisation
# Do not edit targets directly — run /init-<type> to implement them

.PHONY: build fmt fmt-check lint test clean install setup release package publish docs help

## help - show available targets
help:
	@grep -E '^## [a-zA-Z_-]+ - ' Makefile | awk 'BEGIN {FS=" - "} {printf "  %-15s %s\n", substr($$1, 4), $$2}'

## build - compile the project
build:
	@echo "build: not implemented — run /init-<type>"
	@exit 1

## fmt - auto-format code
fmt:
	@echo "fmt: not implemented — run /init-<type>"
	@exit 1

## fmt-check - check code formatting without modifying files
fmt-check:
	@echo "fmt-check: not implemented — run /init-<type>"
	@exit 1

## lint - run formatter check and linter
lint:
	@echo "lint: not implemented — run /init-<type>"
	@exit 1

## test - run the test suite
test:
	@echo "test: not implemented — run /init-<type>"
	@exit 1

## clean - remove build artifacts
clean:
	@echo "clean: not implemented — run /init-<type>"
	@exit 1

## install - install the project locally
install:
	@echo "install: not implemented — run /init-<type>"
	@exit 1

## setup - install all tools and dependencies required to work on this project
setup:
	@echo "setup: not implemented — run /init-<type>"
	@exit 1

## release - tag and trigger the release pipeline
release:
	@echo "release: not implemented — run /init-<type>"
	@exit 1

## package - build distribution packages without releasing
package:
	@echo "package: not implemented — run /init-<type>"
	@exit 1

## publish - publish to package registry
publish:
	@echo "publish: not implemented — run /init-<type>"
	@exit 1

## docs - generate documentation
docs:
	@echo "docs: not implemented — run /init-<type>"
	@exit 1

# ── Project-specific targets ──────────────────────────────────────────────────
# Add targets below that are unique to this project. They will appear in
# `make help` automatically if you use the `## target - description` convention.
# Examples: database migrations, code generation, deployment steps, dev server.
