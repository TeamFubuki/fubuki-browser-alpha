SHELL := /bin/bash

CEF_ROOT ?= $(CURDIR)/third_party/cef
NATIVE_BUILD_DIR ?= $(CURDIR)/native/build
BUILD_TYPE ?= Release
LLVM_PREFIX ?= $(shell brew --prefix llvm 2>/dev/null || echo "/opt/homebrew/opt/llvm")
CLANG_FORMAT := $(LLVM_PREFIX)/bin/clang-format
CLANG_TIDY := $(LLVM_PREFIX)/bin/clang-tidy

.PHONY: help all bootstrap cef ui configure native build run test test-ui test-native lint lint-fix format format-check lint-native format-native lint-all format-all clean distclean

help:
	@echo "Fubuki Browser Alpha"
	@echo ""
	@echo "Targets:"
	@echo "  make bootstrap    Download CEF, install UI dependencies, build UI"
	@echo "  make cef          Download or update CEF into third_party/cef"
	@echo "  make ui           Build the SolidJS UI"
	@echo "  make configure    Configure native CMake build"
	@echo "  make native       Build native app"
	@echo "  make build        Build UI and native app"
	@echo "  make run          Build and run the app"
	@echo "  make test         Run all tests (UI + native)"
	@echo "  make test-ui      Run Vitest (UI)"
	@echo "  make test-native  Build & run GoogleTest (native)"
	@echo "  make lint         Run Oxlint linter (UI)"
	@echo "  make lint-fix     Run Oxlint with auto-fix (UI)"
	@echo "  make format       Run Oxfmt formatter (UI)"
	@echo "  make format-check Check formatting with Oxfmt (UI)"
	@echo "  make lint-native  Run Clang-Tidy linter (C++)"
	@echo "  make format-native Run Clang-Format (C++)"
	@echo "  make lint-all     Run all linters (UI + C++)"
	@echo "  make format-all   Run all formatters (UI + C++)"
	@echo "  make clean        Remove build outputs"
	@echo ""
	@echo "Variables:"
	@echo "  CEF_ROOT=$(CEF_ROOT)"
	@echo "  NATIVE_BUILD_DIR=$(NATIVE_BUILD_DIR)"
	@echo "  BUILD_TYPE=$(BUILD_TYPE)"

all: build

bootstrap:
	@CEF_ROOT="$(CEF_ROOT)" BUILD_TYPE="$(BUILD_TYPE)" ./scripts/bootstrap.sh

cef:
	@CEF_ROOT="$(CEF_ROOT)" ./scripts/fetch_cef.sh

ui:
	@./scripts/build_ui.sh

configure:
	@CEF_ROOT="$(CEF_ROOT)" NATIVE_BUILD_DIR="$(NATIVE_BUILD_DIR)" BUILD_TYPE="$(BUILD_TYPE)" ./scripts/configure_native.sh

native:
	@CEF_ROOT="$(CEF_ROOT)" NATIVE_BUILD_DIR="$(NATIVE_BUILD_DIR)" BUILD_TYPE="$(BUILD_TYPE)" ./scripts/build_native.sh

build:
	@CEF_ROOT="$(CEF_ROOT)" NATIVE_BUILD_DIR="$(NATIVE_BUILD_DIR)" BUILD_TYPE="$(BUILD_TYPE)" ./scripts/build.sh

run:
	@CEF_ROOT="$(CEF_ROOT)" NATIVE_BUILD_DIR="$(NATIVE_BUILD_DIR)" BUILD_TYPE="$(BUILD_TYPE)" ./scripts/run.sh

clean:
	@./scripts/clean.sh

test: test-ui test-native

test-ui:
	@cd ui && pnpm test

test-native:
	@if [ ! -f native/tests/build/CMakeCache.txt ]; then \
		echo "Configuring native tests..."; \
		cmake -S native/tests -B native/tests/build -DCMAKE_BUILD_TYPE=Debug; \
	else \
		echo "Using existing native test build."; \
	fi
	@cmake --build native/tests/build
	@cd native/tests/build && ctest --output-on-failure

lint:
	@cd ui && pnpm lint

lint-fix:
	@cd ui && pnpm lint:fix

format:
	@cd ui && pnpm format

format-check:
	@cd ui && pnpm oxfmt src/ --check

lint-native:
	@echo "Running Clang-Tidy on native source..."
	@find native/src -name '*.cc' -o -name '*.cpp' -o -name '*.h' | xargs $(CLANG_TIDY) -p native/build --quiet 2>/dev/null || true

format-native:
	@echo "Running Clang-Format on native source..."
	@find native/src -name '*.cc' -o -name '*.cpp' -o -name '*.h' | xargs $(CLANG_FORMAT) -i

lint-all: lint lint-native

format-all: format format-native

distclean: clean
	@rm -rf "$(CEF_ROOT)" "$(CURDIR)/.cache" "$(CURDIR)/native/tests/build"
