APP_DIR := app

.PHONY: run run-keychain build release check clippy fmt test clean cargo

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