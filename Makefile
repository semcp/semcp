$(shell [ -d .makes ] || \
  (git clone -q https://github.com/makeplus/makes .makes))
include .makes/init.mk
include $(MAKES)/rust.mk

define HELP
Available targets:
  build        - Build in debug mode
  release      - Build in release mode
  install      - Install to $$PREFIX/bin/
                 PREFIX defaults:
		 - ~/.local/bin
                 - /usr/local/bin (when root)
  clean        - Clean build artifacts
  test         - Run tests
  fmt          - Format code
  check        - Check code without building
  lint|clippy  - Run clippy linter
  help         - Show this help message
endef

RELEASE-FILE := target/release/snpx

PREFIX ?= $(if $(IS-ROOT),/usr/local/bin,$(HOME)/.local)
export BIN := $(PREFIX)/bin


default:: help

help: _makes-help

$(CARGO-CMDS):: $(CARGO)
	cargo $@

lint: clippy

clean::
	$(RM) -r target

release: $(RELEASE-FILE)

$(RELEASE-FILE): $(CARGO)
	cargo build --release
	touch $@

install: $(RELEASE-FILE)
	$(if $(wildcard $(BIN)),,mkdir -p $(BIN))
	cp $< $(BIN)/
	@[[ :$$PATH: == *:$$BIN:* ]] || \
	  echo "Make sure '$$BIN' is in your PATH"
