# SPDX-License-Identifier: CC0-1.0
# socket-mux build entry points: cargo for native builds, cargo-zigbuild with zig
# for the release targets. Tools come from PATH; `make toolchain-check` names any
# missing dependency. Targets: `make help`. Default: the release target native to
# the machine running make (uname -s/-m).

SHELL := /bin/bash

BIN ?= socket-mux

CARGO        ?= cargo
ZIGBUILD     ?= $(CARGO) zigbuild
CARGO_FLAGS  ?=

# Minimum versions this Makefile is verified with ("or newer").
RUST_MIN     := 1.98
ZIG_MIN      := 0.17
ZIGBUILD_MIN := 0.24

# Triples.
T_LINUX_X64   := x86_64-unknown-linux-musl
T_LINUX_ARM64 := aarch64-unknown-linux-musl
T_DARWIN_ARM64 := aarch64-apple-darwin
T_DARWIN_UNIV := universal2-apple-darwin
T_WIN64       := x86_64-pc-windows-gnu
# universal2 is built by cargo-zigbuild from both Apple triples.
XC_RUST_TARGETS := $(T_LINUX_X64) $(T_LINUX_ARM64) $(T_DARWIN_ARM64) x86_64-apple-darwin $(T_WIN64)

# Default goal: the release target native to the machine running make. Unknown
# platforms fall back to help.
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)
NATIVE_TARGET := $(strip \
  $(if $(filter Linux,$(UNAME_S)), \
    $(if $(filter x86_64 amd64,$(UNAME_M)),linux-x86_64-musl, \
      $(if $(filter aarch64 arm64,$(UNAME_M)),linux-aarch64-musl)), \
    $(if $(filter Darwin,$(UNAME_S)), \
      $(if $(filter arm64 aarch64,$(UNAME_M)),darwin-aarch64))))
.DEFAULT_GOAL := $(or $(NATIVE_TARGET),help)

.PHONY: help toolchain-help toolchain-check \
	dev check test clean \
	all linux darwin win64 \
	linux-x86_64-musl linux-aarch64-musl \
	darwin-aarch64 darwin-universal2 \
	win64-x86_64 \
	darwin-run-help win64-run-help darwin-sign-adhoc

help:
	@echo "socket-mux make targets"
	@echo ""
	@echo "  (default)            = $(or $(NATIVE_TARGET),help)  (native to $(UNAME_S) $(UNAME_M))"
	@echo "  linux-x86_64-musl    release, $(T_LINUX_X64)   (cargo zigbuild)"
	@echo "  linux-aarch64-musl   release, $(T_LINUX_ARM64)  (cargo zigbuild)"
	@echo "  linux                both linux musl targets"
	@echo "  darwin-aarch64       release, $(T_DARWIN_ARM64)        (cargo zigbuild, unsigned)"
	@echo "  darwin-universal2    release, $(T_DARWIN_UNIV)     (cargo zigbuild, unsigned)"
	@echo "  darwin               both darwin targets"
	@echo "  win64-x86_64         release, $(T_WIN64)       (cargo zigbuild, needs Win10+ UCRT)"
	@echo "  win64                = win64-x86_64"
	@echo "  all                  every release binary above (linux + darwin + win64)"
	@echo "  dev                  native debug build (cargo build)"
	@echo "  check / test         cargo check / cargo test (native)"
	@echo "  clean                cargo clean"
	@echo "  toolchain-check      verify cargo, rustc, zig, cargo-zigbuild and the rust targets"
	@echo "  toolchain-help       how to install the build dependencies"
	@echo "  darwin-run-help      how to run the Mac binaries   (NOT YET TESTED)"
	@echo "  win64-run-help       how to run the Windows binary (NOT YET TESTED)"
	@echo "  darwin-sign-adhoc    ad-hoc codesign SIGN_BIN=<path>, on macOS (NOT YET TESTED)"
	@echo ""
	@echo "Variables: BIN=$(BIN)  CARGO=$(CARGO)  CARGO_FLAGS=$(CARGO_FLAGS)"

toolchain-help:
	@echo "Build dependencies (versions verified, or newer):"
	@echo "  rust $(RUST_MIN)+ (rustc, cargo), e.g. via rustup: https://rustup.rs"
	@echo "  zig $(ZIG_MIN)+ on PATH: https://ziglang.org/download/"
	@echo "  cargo-zigbuild $(ZIGBUILD_MIN)+: cargo install --locked cargo-zigbuild"
	@echo "  rust targets for the release builds you want:"
	@echo "    rustup target add $(XC_RUST_TARGETS)"
	@echo "Native builds (dev, check, test) need only rust."

# deps_check <tools> <rust targets>: fail with a message naming each missing dependency.
define deps_check
	@fail=0; \
	for d in $(1); do command -v $$d >/dev/null 2>&1 || { echo "error: $$d not found on PATH (need it for this target; see: make toolchain-help)" >&2; fail=1; }; done; \
	if [ -n "$(strip $(2))" ] && command -v rustc >/dev/null 2>&1; then \
	  sysroot=$$(rustc --print sysroot); \
	  for t in $(2); do [ -d "$$sysroot/lib/rustlib/$$t" ] || { echo "error: rust target $$t not installed (rustup target add $$t)" >&2; fail=1; }; done; \
	fi; \
	exit $$fail
endef

NATIVE_DEPS := cargo rustc
XC_DEPS     := cargo rustc zig cargo-zigbuild

toolchain-check:
	$(call deps_check,$(XC_DEPS),$(XC_RUST_TARGETS))
	@echo "ok: $$(rustc --version); zig $$(zig version); $$(cargo-zigbuild --version)"

# ---- native builds -------------------------------------------------------
dev:
	$(call deps_check,$(NATIVE_DEPS),)
	$(CARGO) build $(CARGO_FLAGS)

check:
	$(call deps_check,$(NATIVE_DEPS),)
	$(CARGO) check $(CARGO_FLAGS)

test:
	$(call deps_check,$(NATIVE_DEPS),)
	$(CARGO) test $(CARGO_FLAGS)

clean:
	$(CARGO) clean

# ---- release builds (cargo zigbuild) -------------------------------------
# Output: target/<triple>/release/$(BIN)   (.exe for win64)
linux-x86_64-musl:
	$(call deps_check,$(XC_DEPS),$(T_LINUX_X64))
	$(ZIGBUILD) --release --target $(T_LINUX_X64) $(CARGO_FLAGS)

linux-aarch64-musl:
	$(call deps_check,$(XC_DEPS),$(T_LINUX_ARM64))
	$(ZIGBUILD) --release --target $(T_LINUX_ARM64) $(CARGO_FLAGS)

darwin-aarch64:
	$(call deps_check,$(XC_DEPS),$(T_DARWIN_ARM64))
	$(ZIGBUILD) --release --target $(T_DARWIN_ARM64) $(CARGO_FLAGS)

darwin-universal2:
	$(call deps_check,$(XC_DEPS),$(T_DARWIN_ARM64) x86_64-apple-darwin)
	$(ZIGBUILD) --release --target $(T_DARWIN_UNIV) $(CARGO_FLAGS)

win64-x86_64:
	$(call deps_check,$(XC_DEPS),$(T_WIN64))
	$(ZIGBUILD) --release --target $(T_WIN64) $(CARGO_FLAGS)

linux: linux-x86_64-musl linux-aarch64-musl
darwin: darwin-aarch64 darwin-universal2
win64: win64-x86_64
all: linux darwin win64

# ---- run on target systems: NOT YET TESTED --------------------------------
darwin-run-help:
	@echo "macOS run steps (NOT YET TESTED), on the Mac:"
	@echo "  1. copy target/$(T_DARWIN_ARM64)/release/$(BIN) (Apple Silicon) or"
	@echo "     target/$(T_DARWIN_UNIV)/release/$(BIN) (Intel + Apple Silicon) to the Mac"
	@echo "  2. if it was downloaded: xattr -d com.apple.quarantine ./$(BIN)"
	@echo "  3. ad-hoc sign (Apple Silicon refuses to run unsigned code): codesign --force --sign - ./$(BIN)"
	@echo "     (or: make darwin-sign-adhoc SIGN_BIN=./$(BIN) on the Mac)"
	@echo "  4. verify: codesign -v ./$(BIN) && codesign -dv ./$(BIN)"
	@echo "  5. run: ./$(BIN)"
	@echo "  Notes: rcodesign (apple-codesign) can ad-hoc sign from Linux instead."
	@echo "         Distribution needs Developer ID signing plus notarization."

win64-run-help:
	@echo "Windows run steps (NOT YET TESTED):"
	@echo "  1. copy target/$(T_WIN64)/release/$(BIN).exe to the Windows machine"
	@echo "     (needs Windows 10+: the exe imports the UCRT api-ms-win-crt-*.dll)"
	@echo "  2. if it was downloaded (Mark of the Web), in PowerShell: Unblock-File .\\$(BIN).exe"
	@echo "  3. SmartScreen may warn because the binary is unsigned"
	@echo "  4. allow the Windows Defender Firewall prompt, or add an inbound rule for the UDP ports, e.g."
	@echo "     New-NetFirewallRule -DisplayName $(BIN) -Direction Inbound -Protocol UDP -LocalPort <ports> -Action Allow"
	@echo "  5. run it from a console (cmd or PowerShell): .\\$(BIN).exe"
	@echo "  Later/optional: Authenticode signing, e.g. with osslsigncode."

# Runs on macOS only; refuses elsewhere. Builds nothing. NOT YET TESTED.
SIGN_BIN ?= ./$(BIN)
darwin-sign-adhoc:
	@if [ "$$(uname -s)" != Darwin ]; then \
	  echo "error: darwin-sign-adhoc runs on macOS only (use rcodesign to sign from Linux)" >&2; exit 1; fi
	@test -f "$(SIGN_BIN)" || { echo "error: $(SIGN_BIN) not found; pass SIGN_BIN=<path>" >&2; exit 1; }
	-xattr -d com.apple.quarantine "$(SIGN_BIN)" 2>/dev/null
	codesign --force --sign - "$(SIGN_BIN)"
	codesign -v "$(SIGN_BIN)"
	codesign -dv "$(SIGN_BIN)"
