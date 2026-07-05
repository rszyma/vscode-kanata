.PHONY: test lint typecheck fmtcheck fmt package build-wasm build-native run-native clean

test:
	# yarn test
	cd kanata-ls; cargo test

typecheck: build-wasm
	yarn typecheck

fmtcheck: clean
	yarn fmtcheck

fmt: clean
	yarn fmtwrite

package: build-wasm
	yarn package

RELEASE ?= 0

build-wasm: clean
ifeq ($(RELEASE),1)
	$(MAKE) CARGO_FLAGS=--release OUT_DIR=$$(pwd)/out -C kanata-ls build-wasm
else
	$(MAKE) CARGO_FLAGS=--dev OUT_DIR=$$(pwd)/out -C kanata-ls build-wasm
endif

build-native: clean
ifeq ($(RELEASE),1)
	$(MAKE) CARGO_FLAGS=--release -C kanata-ls build-native
else
	$(MAKE) -C kanata-ls build-native
endif

run-native: clean
ifeq ($(RELEASE),1)
	$(MAKE) CARGO_FLAGS=--release -C kanata-ls run-native
else
	$(MAKE) -C kanata-ls run-native
endif

node_modules: package.json
	yarn install
	@touch $@

clean: node_modules
	rm -rf out
	mkdir -p out