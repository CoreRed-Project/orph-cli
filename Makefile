BIN_NAME  = orph
DAEMON    = orphd
TARGET_ARM = aarch64-unknown-linux-gnu
RELEASE_DIR = target/release
ARM_DIR     = target/$(TARGET_ARM)/release

.PHONY: build release cross install clean fmt lint

build:
	cargo build

release:
	cargo build --release

cross:
	cargo build --release --target $(TARGET_ARM)

install: release
	install -m 755 $(RELEASE_DIR)/$(BIN_NAME)  /usr/local/bin/$(BIN_NAME)
	install -m 755 $(RELEASE_DIR)/$(DAEMON)     /usr/local/bin/$(DAEMON)

fmt:
	cargo fmt --all

lint:
	cargo clippy --all-targets -- -D warnings

clean:
	cargo clean
