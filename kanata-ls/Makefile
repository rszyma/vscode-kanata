.PHONY: build-native build-wasm test-wasm require-out-dir

run-native:
	cargo run $(CARGO_FLAGS)

build-native:
	cargo build $(CARGO_FLAGS)

build-wasm: require-out-dir
	wasm-pack build $${CARGO_FLAGS:---dev} --target nodejs --out-dir $(OUT_DIR)
	rm -f $(OUT_DIR)/.gitignore $(OUT_DIR)/package.json

require-out-dir:
ifndef OUT_DIR
	$(error OUT_DIR is not defined)
endif