SHELL := bash

define HELP
Available targets:
  build        - Build in debug mode
  release      - Build in release mode
  install      - Install to ~/.local/bin
                   or to /usr/local/bin (with sudo)
  clean        - Clean build artifacts
  test         - Run tests
  fmt          - Format code
  check        - Check code without building
  lint|clippy  - Run clippy linter
  help         - Show this help message
endef
export HELP

RELEASE-FILE := target/release/snpx


default: build

build:
	cargo build

release: $(RELEASE-FILE)

$(RELEASE-FILE):
	cargo build --release

install: $(RELEASE-FILE)
ifeq (0,$(shell id -u))
	sudo cp $< /usr/local/bin/snpx
else
	mkdir -p ~/.local/bin
	cp $< ~/.local/bin/snpx
	@[[ :$$PATH: == *:$$HOME/.local/bin:* ]] || \
	  echo 'Make sure ~/.local/bin is in your PATH'
endif

clean test fmt check clippy:
	cargo $@

lint:
	cargo clippy

help:
	@echo "$$HELP"
