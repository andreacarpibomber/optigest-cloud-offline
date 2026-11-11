APP_DIR := app
VERSION := $(shell grep -m1 '^version' $(APP_DIR)/Cargo.toml | cut -d'"' -f2)
WIN_TARGET := x86_64-pc-windows-gnu
WIN_OUT := $(APP_DIR)/target/$(WIN_TARGET)/release

.PHONY: run run-keychain build release check clippy fmt test clean cargo build-windows create_deliverable exe

# Sviluppo: segreti su file (niente richieste di password del keychain), solo build debug.
run:
	cd $(APP_DIR) && OPTIGEST_DEV_STORE=file cargo run

# Come run, ma con il keychain di sistema.
run-keychain:
	cd $(APP_DIR) && cargo run

build:
	cd $(APP_DIR) && cargo build

release:
	cd $(APP_DIR) && cargo build --release

check:
	cd $(APP_DIR) && cargo check

clippy:
	cd $(APP_DIR) && cargo clippy

fmt:
	cd $(APP_DIR) && cargo fmt

test:
	cd $(APP_DIR) && cargo test

clean:
	cd $(APP_DIR) && cargo clean

# Comando libero: make cargo ARGS="tree"
cargo:
	cd $(APP_DIR) && cargo $(ARGS)

# Windows (cross-compilazione da Mac, serve: brew install mingw-w64)
build-windows:
	@rustup target list --installed | grep -q '^$(WIN_TARGET)$$' || rustup target add -q $(WIN_TARGET)
	cd $(APP_DIR) && cargo build --release --target $(WIN_TARGET) --features custom-protocol

create_deliverable: build-windows
	rm -rf deliverable
	mkdir -p deliverable
	cp $(WIN_OUT)/eos_desktop.exe "deliverable/OptiGest Cloud Offline.exe"
	cp "$$(find $(APP_DIR)/target/$(WIN_TARGET) -name WebView2Loader.dll | head -n 1)" deliverable/
	cp $(APP_DIR)/config.json deliverable/
	cd deliverable && zip -r ../optigest-cloud-offline-$(VERSION)-win.zip . -x "*/.DS_Store" "*/Thumbs.db"
	rm -rf deliverable

# Alias
exe: create_deliverable