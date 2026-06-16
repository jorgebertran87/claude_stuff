IMAGE := prices-comparer
SHELL := /bin/bash

ifneq (,$(wildcard ./.env))
    include .env
    export
endif

# Ensure the shared DeepSeek crate is available inside the Docker build context.
_shared = rsync -a --delete --exclude=target ../deepseek_client/ ./deepseek_client/ &&

build-prod:
	$(_shared) docker buildx build --platform $(PROD_PLATFORM) \
		-t $(DOCKER_USERNAME)/$(IMAGE) --push .

UNIT_TESTS        = $(patsubst tests/unit/%.rs,--test %,$(wildcard tests/unit/*.rs))
INTEGRATION_TESTS = $(patsubst tests/integration/%.rs,--test %,$(wildcard tests/integration/*.rs))

# Map infrastructure source file → relevant integration test (name matches file stem)
FILE_TESTS = $(if $(FILE),--test $(basename $(notdir $(FILE))),$(INTEGRATION_TESTS))

build:
	$(_shared) docker build --target runtime -t $(IMAGE) .

test:
	$(_shared) docker build --target test -t $(IMAGE)-test .
	docker run --rm $(IMAGE)-test cargo test $(UNIT_TESTS)

test-integration:
	$(_shared) docker build --target test -t $(IMAGE)-test .
	docker run --rm $(IMAGE)-test cargo test $(INTEGRATION_TESTS)

test-all:
	$(_shared) docker build --target test -t $(IMAGE)-test .
	docker run --rm $(IMAGE)-test cargo test $(UNIT_TESTS) $(INTEGRATION_TESTS)

mutants:
	$(_shared) docker build --target mutants -t $(IMAGE)-mutants .
	docker run --rm $(IMAGE)-mutants \
		cargo mutants -j 3 --config config/mutants.toml -- $(UNIT_TESTS)

mutants-integration:
	$(_shared) docker build --target mutants -t $(IMAGE)-mutants .
	docker run --rm $(IMAGE)-mutants \
		cargo mutants -j 3 --config config/mutants-integration.toml -- $(FILE_TESTS)

run:
	docker compose up --build -d
	docker compose logs -f

run-prod:
	RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) docker compose up --no-build -d
	docker compose logs -f

deploy:
	$(MAKE) test
	$(MAKE) build-prod
	ssh pequenin 'mkdir -p ~/prices_comparer/capture'
	scp Makefile docker-compose.yml pequenin:~/prices_comparer
	scp capture/glovo_token.py pequenin:~/prices_comparer/capture
	scp -r .claude pequenin:~/prices_comparer/

.PHONY: build build-prod test test-integration test-all mutants mutants-integration run run-prod deploy
