.PHONY: build release install clean test fmt check

all: build

build:
	cargo build

release:
	cargo build --release

install: release
	sudo cp target/release/snpx /usr/local/bin/snpx

install-user: release
	mkdir -p ~/.local/bin
	cp target/release/snpx ~/.local/bin/snpx
	@echo "Make sure ~/.local/bin is in your PATH"

clean:
	cargo clean

test:
	cargo test

fmt:
	cargo fmt

check:
	cargo check

lint:
	cargo clippy

help:
	@echo "Available targets:"
	@echo "  build        - Build in debug mode"
	@echo "  release      - Build in release mode"
	@echo "  install      - Install to /usr/local/bin (requires sudo)"
	@echo "  install-user - Install to ~/.local/bin"
	@echo "  clean        - Clean build artifacts"
	@echo "  test         - Run tests"
	@echo "  fmt          - Format code"
	@echo "  check        - Check code without building"
	@echo "  lint         - Run clippy linter"
	@echo "  help         - Show this help message" 