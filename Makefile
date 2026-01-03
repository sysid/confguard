.DEFAULT_GOAL := help
#MAKEFLAGS += --no-print-directory

# You can set these variables from the command line, and also from the environment for the first two.
PREFIX ?= /usr/local
BINPREFIX ?= "$(PREFIX)/bin"

VERSION       = $(shell cat VERSION)

SHELL	= bash
.ONESHELL:

app_root := $(if $(PROJ_DIR),$(PROJ_DIR),$(CURDIR))
pkg_src =  $(app_root)/confguard
tests_src = $(app_root)/confguard/tests
BINARY = confguard

# Makefile directory
CODE_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# define files
MANS = $(wildcard ./*.md)
MAN_HTML = $(MANS:.md=.html)
MAN_PAGES = $(MANS:.md=.1)

################################################################################
# Admin \
ADMIN::  ## ##################################################################
.PHONY: init-env
init-env:  ## init-env
	@rm -fr ~/xxx/confguard-test/*
	@mkdir -p ~/xxx/confguard-test
	@echo 'export FOO=bar' > ~/xxx/confguard-test/.envrc
	@echo "Test environment created at ~/xxx/confguard-test"

.PHONY: show-env
show-env:  ## show-env
	@tree -a ~/xxx/confguard-test 2>/dev/null || ls -la ~/xxx/confguard-test

.PHONY: test
test:  ## test
	RUST_LOG=DEBUG pushd $(pkg_src) && cargo test -- --test-threads=1
	#RUST_LOG=DEBUG pushd $(pkg_src) && cargo test

.PHONY: run-guard
run-guard: init-env  ## run-guard: test guarding a project
	pushd $(pkg_src) && cargo run -- guard ~/xxx/confguard-test

.PHONY: run-show
run-show:  ## run-show: show guarded project info
	pushd $(pkg_src) && cargo run -- show ~/xxx/confguard-test

.PHONY: run-unguard
run-unguard:  ## run-unguard: unguard a project
	pushd $(pkg_src) && cargo run -- unguard ~/xxx/confguard-test

.PHONY: run-init
run-init:  ## run-init: initialize new .envrc
	@mkdir -p ~/xxx/confguard-init-test
	pushd $(pkg_src) && cargo run -- init ~/xxx/confguard-init-test

.PHONY: run-settings
run-settings:  ## run-settings: show confguard settings
	pushd $(pkg_src) && cargo run -- settings

.PHONY: run-sops-init
run-sops-init:  ## run-sops-init: initialize SOPS configuration
	pushd $(pkg_src) && cargo run -- sops-init

.PHONY: test-sops
test-sops:  ## test-sops: test SOPS functionality (requires gpg setup)
	pushd $(pkg_src) && cargo test sops -- --nocapture --ignored

.PHONY: test-env-vars
test-env-vars:  ## test-env-vars: test environment variable resolution in confguard comments
	@echo "=== Testing Environment Variable Resolution ==="
	@echo "Setting up CONFGUARD_TEST_ROOT variable..."
	@export CONFGUARD_TEST_ROOT=$(tests_src)/resources/data && \
	echo "CONFGUARD_TEST_ROOT=$$CONFGUARD_TEST_ROOT" && \
	echo "" && \
	echo "=== Testing guard command ===" && \
	pushd $(pkg_src) && cargo run -- guard tests/resources/data/testprj && \
	echo "" && \
	echo "=== Testing show command ===" && \
	pushd $(pkg_src) && cargo run -- show tests/resources/data/testprj && \
	echo "" && \
	echo "✓ All tests completed successfully!"

################################################################################
# Building, Deploying \
BUILDING:  ## ##################################################################

.PHONY: doc
doc:  ## doc
	@rustup doc --std
	pushd $(pkg_src) && cargo doc --open

.PHONY: all
all: clean build install  ## all
	:

.PHONY: upload
upload:  ## upload
	@if [ -z "$$CARGO_REGISTRY_TOKEN" ]; then \
		echo "Error: CARGO_REGISTRY_TOKEN is not set"; \
		exit 1; \
	fi
	@echo "CARGO_REGISTRY_TOKEN is set"
	pushd $(pkg_src) && cargo release publish --execute

.PHONY: build
build:  ## build
	pushd $(pkg_src) && cargo build --release

.PHONY: install
install: uninstall  ## install
	@VERSION=$(VERSION) && \
		echo "-M- Installing $$VERSION" && \
		cp -vf $(pkg_src)/target/release/$(BINARY) ~/bin/$(BINARY)$$VERSION && \
		ln -vsf ~/bin/$(BINARY)$$VERSION ~/bin/$(BINARY)

.PHONY: uninstall
uninstall:  ## uninstall
	-@test -f ~/bin/$(BINARY) && rm -v ~/bin/$(BINARY)

.PHONY: bump-major
bump-major: check-github-token  ## bump-major, tag and push
	bump-my-version bump --commit --tag major
	git push
	git push --tags
	@$(MAKE) create-release

.PHONY: bump-minor
bump-minor: check-github-token  ## bump-minor, tag and push
	bump-my-version bump --commit --tag minor
	git push
	git push --tags
	@$(MAKE) create-release

.PHONY: bump-patch
bump-patch: check-github-token  ## bump-patch, tag and push
	bump-my-version bump --commit --tag patch
	git push
	git push --tags
	@$(MAKE) create-release

.PHONY: create-release
create-release: check-github-token  ## create a release on GitHub via the gh cli
	@if ! command -v gh &>/dev/null; then \
		echo "You do not have the GitHub CLI (gh) installed. Please create the release manually."; \
		exit 1; \
	else \
		echo "Creating GitHub release for v$(VERSION)"; \
		gh release create "v$(VERSION)" --generate-notes --latest; \
	fi

.PHONY: check-github-token
check-github-token:  ## Check if GITHUB_TOKEN is set
	@if [ -z "$$GITHUB_TOKEN" ]; then \
		echo "GITHUB_TOKEN is not set. Please export your GitHub token before running this command."; \
		exit 1; \
	fi
	@echo "GITHUB_TOKEN is set"

.PHONY: fix-version
fix-version:  ## fix-version of Cargo.toml, re-connect with HEAD
	git add $(pkg_src)/Cargo.lock
	git commit --amend --no-edit
	git tag -f "v$(VERSION)"
	git push --force-with-lease
	git push --tags --force

.PHONY: style
style:  ## style
	pushd $(pkg_src) && cargo fmt

.PHONY: lint
lint:  ## lint
	pushd $(pkg_src) && cargo clippy

################################################################################
# Clean \
CLEAN:  ## ############################################################

.PHONY: clean
clean:clean-rs  ## clean all
	:

.PHONY: clean-build
clean-build: ## remove build artifacts
	rm -fr build/
	rm -fr dist/
	rm -fr .eggs/
	find . \( -path ./env -o -path ./venv -o -path ./.env -o -path ./.venv \) -prune -o -name '*.egg-info' -exec rm -fr {} +
	find . \( -path ./env -o -path ./venv -o -path ./.env -o -path ./.venv \) -prune -o -name '*.egg' -exec rm -f {} +

.PHONY: clean-pyc
clean-pyc: ## remove Python file artifacts
	find . -name '*.pyc' -exec rm -f {} +
	find . -name '*.pyo' -exec rm -f {} +
	find . -name '*~' -exec rm -f {} +
	find . -name '__pycache__' -exec rm -fr {} +

.PHONY: clean-rs
clean-rs:  ## clean-rs
	pushd $(pkg_src) && cargo clean -v

################################################################################
# Misc \
MISC:  ## ############################################################

define PRINT_HELP_PYSCRIPT
import re, sys

for line in sys.stdin:
	match = re.match(r'^([%a-zA-Z0-9_-]+):.*?## (.*)$$', line)
	if match:
		target, help = match.groups()
		if target != "dummy":
			print("\033[36m%-20s\033[0m %s" % (target, help))
endef
export PRINT_HELP_PYSCRIPT

.PHONY: help
help:
	@python -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

debug:  ## debug
	@echo "-D- CODE_DIR: $(CODE_DIR)"
	@echo "-D- VERSION: $(VERSION)"
	@echo "-D- BINARY: $(BINARY)"
	@echo "-D- app_root: $(app_root)"

.PHONY: list
list: *  ## list
	@echo $^

.PHONY: list2
%: %.md  ## list2
	@echo $^

%-plan:  ## call with: make <whatever>-plan
	@echo $@ : $*
	@echo $@ : $^