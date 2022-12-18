.DEFAULT_GOAL := help
#MAKEFLAGS += --no-print-directory

# You can set these variables from the command line, and also from the environment for the first two.
VERSION       = $(shell cat VERSION)

SHELL	= bash
.ONESHELL:

MAKE    = make
PYTHON	= python
PYTEST	= pytest --log-level=debug --capture=tee-sys --asyncio-mode=auto
PYTOPT	=
VENV	= venv
PIP		= venv/bin/pip
PACKAGE = $confguard

app_root = .
app_root ?= .
pkg_src =  $(app_root)/src
tests_src = $(app_root)/tests

.PHONY: all
all: clean build upload tag  ## Build and upload
	@echo "--------------------------------------------------------------------------------"
	@echo "-M- building and distributing"
	@echo "--------------------------------------------------------------------------------"

################################################################################
# Testing \
TESTING:  ## ############################################################

.PHONY: coverage
coverage:  ## Run tests with coverage
	python -m coverage erase
	python -m coverage run --include=$(pkg_src)/* -m pytest -ra
	python -m coverage report -m
	python -m xml

.PHONY: test
test:  ## run tests
	python -m pytest -ra --junitxml=report.xml --cov-config=setup.cfg --cov-report=xml --cov-report term --cov=$(pkg_src) -vv tests/

.PHONY: tox
tox:   ## Run tox
	tox

################################################################################
# Building, Deploying \
building:  ## ##################################################################

.PHONY: build
build: clean format isort  ## format and build
	@echo "building"
	python -m build

.PHONY: dist
dist:  ## - create a wheel distribution package
	@python setup.py bdist_wheel

.PHONY: dist-test
dist-test: dist  ## - test a wheel distribution package
	@cd dist && ../tests/test-dist.bash ./confguard-*-py3-none-any.whl

.PHONY: install
install: uninstall
	pipx install $(app_root)

.PHONY: uninstall
uninstall:  ## pipx uninstall
	-pipx uninstall $(PACKAGE)

.PHONY: bump-major
bump-major:  ## bump-major, tag and push
	bumpversion --commit --tag major
	git push --tags

.PHONY: bump-minor
bump-minor:  ## bump-minor, tag and push
	bumpversion --commit --tag minor
	git push --tags

.PHONY: bump-patch
bump-patch:  ## bump-patch, tag and push
	bumpversion --commit --tag patch
	git push --tags
	#git push  # triggers additional build, but no code change (for bumping workspace must be clean)

.PHONY: upload
upload:  ## upload to PyPi
	twine upload --verbose dist/*

.PHONY: upload-test
upload-test:  ## upload to Test-PyPi
	twine upload --repository testpypi dist/*

.PHONY: dist-upload
dist-upload:  ## - upload a wheel distribution package
	@twine upload dist/confguard-*-py3-none-any.whl

################################################################################
# Code Quality \
QUALITY:  ## ############################################################

.PHONY: style
style: isort format  ## perform code style format (black, isort)

.PHONY: format
format:  ## perform black formatting
	black $(pkg_src) tests

.PHONY: isort
isort:  ## apply import sort ordering
	isort . --profile black

.PHONY: lint
lint: flake8 mypy ## lint code with all static code checks

.PHONY: flake8
flake8:  ## check style with flake8
	@flake8 $(pkg_src)

.PHONY: mypy
mypy:  ## check type hint annotations
	# keep config in setup.cfg for integration with PyCharm
	mypy --config-file setup.cfg $(pkg_src)

.PHONY: complexity
complexity:  ## measure complexity KPIs
	radon cc --show-complexity --min C --exclude '**/buku*' $(pkg_src)

.PHONY: pyroma
pyroma:  ## measure package best practice compliance
	pyroma --min 9 .


################################################################################
# Documenation \
DOCU:  ## ############################################################

.PHONY: docs
docs: coverage  ## - generate project documentation
	@cd docs; rm -rf source/api/confguard*.rst source/api/modules.rst build/*
	@cd docs; make html

.PHONY: check-docs
check-docs:  ## - quick check docs consistency
	@cd docs; make dummy

.PHONY: serve-docs
serve-docs:  ## - serve project html documentation
	@cd docs/build; python -m http.server --bind 127.0.0.1


################################################################################
# Clean \
CLEAN:  ## ############################################################

.PHONY: clean
clean: clean-build clean-pyc  ## remove all build, test, coverage and Python artifacts

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


################################################################################
# Misc \
MISC:  ## ############################################################

define PRINT_HELP_PYSCRIPT
import re, sys

for line in sys.stdin:
	match = re.match(r'^([a-zA-Z0-9_-]+):.*?## (.*)$$', line)
	if match:
		target, help = match.groups()
		print("\033[36m%-20s\033[0m %s" % (target, help))
endef
export PRINT_HELP_PYSCRIPT

.PHONY: help
help:
	@python -c "$$PRINT_HELP_PYSCRIPT" < $(MAKEFILE_LIST)

.PHONY: venv
venv:  ## - create a virtual environment for development
	@rm -Rf venv
	@python3 -m venv venv --prompt confguard
	@/bin/bash -c "source venv/bin/activate && pip install pip --upgrade && pip install -r requirements.dev.txt && pip install -e ."
	@echo "Enter virtual environment using:\n\n\t$ source venv/bin/activate\n"

.PHONY: deps
deps:  ## Install dependencies (ensure to be in venv)
	python -m pip install --upgrade pip
	python -m pip install black coverage mypy pylint pytest tox tox-gh-actions

.PHONY: venv-dev
venv-dev: venv ## install in venv with dev deps
	$(PIP) install -r dev-requirements.txt

.PHONY: venv-prod
venv-prod: venv  ## install in venv

.PHONY: venv-last
venv-last:  ## install in venv.last
	$(PIP) install $$($(PIP) freeze | cut -d= -f1 | grep -v -- '^-e') -U
