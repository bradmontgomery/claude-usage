BIN         := claude-usage
HOOK_BIN    := collect-session-stats
INSTALL_DIR := $(HOME)/.local/bin

.PHONY: build release run check install install-hook install-all clean help

help:
	@echo "Targets:"
	@echo "  install-hook   build + install both binaries, register SessionEnd hook"
	@echo "  install        build release binaries and copy to $(INSTALL_DIR)"
	@echo "  install-all    alias for install-hook"
	@echo "  build          debug build"
	@echo "  release        optimized build"
	@echo "  run            cargo run (pass args after --)"
	@echo "  check          fast compile check, no binary"
	@echo "  clean          remove target/"

build:
	cargo build

release:
	cargo build --release

run:
	cargo run --

check:
	cargo check

install: release
	mkdir -p $(INSTALL_DIR)
	cp target/release/$(BIN) $(INSTALL_DIR)/$(BIN)
	cp target/release/$(HOOK_BIN) $(INSTALL_DIR)/$(HOOK_BIN)
	@echo "Installed to $(INSTALL_DIR)"

install-hook: install
	python3 hook/register.py $(INSTALL_DIR)/$(HOOK_BIN)

install-all: install-hook

clean:
	cargo clean
