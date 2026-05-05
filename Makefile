BIN         := claude-usage
INSTALL_DIR := $(HOME)/.local/bin
CLAUDE_DIR  := $(HOME)/.claude

.PHONY: build release run check install install-hook install-all clean help

help:
	@echo "Targets:"
	@echo "  install-hook   copy hook script + register SessionEnd hook in ~/.claude/settings.json"
	@echo "  install        build release binary and copy to $(INSTALL_DIR)"
	@echo "  install-all    install-hook + install"
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
	@echo "Installed to $(INSTALL_DIR)/$(BIN)"

install-hook:
	mkdir -p $(CLAUDE_DIR)
	cp hook/collect-session-stats.py $(CLAUDE_DIR)/collect-session-stats.py
	python3 hook/register.py

install-all: install-hook install

clean:
	cargo clean
