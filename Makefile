SHELL := bash

define HELP
Available targets:
  build        - Build all workspace members in debug mode
  release      - Build all workspace members in release mode
  install      - Install all binaries to ~/.local/bin
                   or to /usr/local/bin (with sudo)
  install-snpx - Install only snpx binary
  install-suvx - Install only suvx binary
  clean        - Clean build artifacts
  test         - Run tests
  fmt          - Format code
  check        - Check code without building
  lint|clippy  - Run clippy linter
  help         - Show this help message
endef
export HELP

RELEASE_DIR := target/release

default: build

build:
	cargo build

release: $(RELEASE_DIR)/snpx $(RELEASE_DIR)/suvx

$(RELEASE_DIR)/snpx:
	cargo build --release -p snpx

$(RELEASE_DIR)/suvx:
	cargo build --release -p suvx

install: $(RELEASE_DIR)/snpx $(RELEASE_DIR)/suvx
ifeq (0,$(shell id -u))
	sudo cp $(RELEASE_DIR)/snpx /usr/local/bin/snpx
	sudo cp $(RELEASE_DIR)/suvx /usr/local/bin/suvx
else
	mkdir -p ~/.local/bin
	cp $(RELEASE_DIR)/snpx ~/.local/bin/snpx
	cp $(RELEASE_DIR)/suvx ~/.local/bin/suvx
	@[[ :$$PATH: == *:$$HOME/.local/bin:* ]] || \
	  echo 'Make sure ~/.local/bin is in your PATH'
endif

install-snpx: $(RELEASE_DIR)/snpx
ifeq (0,$(shell id -u))
	sudo cp $(RELEASE_DIR)/snpx /usr/local/bin/snpx
else
	mkdir -p ~/.local/bin
	cp $(RELEASE_DIR)/snpx ~/.local/bin/snpx
	@[[ :$$PATH: == *:$$HOME/.local/bin:* ]] || \
	  echo 'Make sure ~/.local/bin is in your PATH'
endif

install-suvx: $(RELEASE_DIR)/suvx
ifeq (0,$(shell id -u))
	sudo cp $(RELEASE_DIR)/suvx /usr/local/bin/suvx
else
	mkdir -p ~/.local/bin
	cp $(RELEASE_DIR)/suvx ~/.local/bin/suvx
	@[[ :$$PATH: == *:$$HOME/.local/bin:* ]] || \
	  echo 'Make sure ~/.local/bin is in your PATH'
endif

clean test fmt check clippy:
	cargo $@

lint:
	cargo clippy

help:
	@echo "$$HELP"
