SHELL := /bin/bash

CEF_ROOT ?= $(CURDIR)/third_party/cef
NATIVE_BUILD_DIR ?= $(CURDIR)/native/build
BUILD_TYPE ?= Release

.PHONY: help all bootstrap cef ui configure native build run test test-ui test-native clean distclean

help:
	@echo "Fubuki Browser Alpha"
	@echo ""
	@echo "Targets:"
	@echo "  make bootstrap   Download CEF, install UI dependencies, build UI"
	@echo "  make cef         Download or update CEF into third_party/cef"
	@echo "  make ui          Build the SolidJS UI"
	@echo "  make configure   Configure native CMake build"
	@echo "  make native      Build native app"
	@echo "  make build       Build UI and native app"
	@echo "  make run         Build and run the app"
	@echo "  make test        Run all tests (UI + native)"
	@echo "  make test-ui     Run Vitest (UI)"
	@echo "  make test-native Build & run GoogleTest (native)"
	@echo "  make clean       Remove build outputs"
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
	@cmake -S native/tests -B native/tests/build -DCMAKE_BUILD_TYPE=Debug
	@cmake --build native/tests/build
	@cd native/tests/build && ctest --output-on-failure

distclean: clean
	@rm -rf "$(CEF_ROOT)" "$(CURDIR)/.cache" "$(CURDIR)/native/tests/build"
