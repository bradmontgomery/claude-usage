BIN := claude-usage
INSTALL_DIR := $(HOME)/.local/bin

.PHONY: build release run check install clean

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

clean:
	cargo clean
