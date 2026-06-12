IMAGE := prices-comparer
SHELL := /bin/bash

ifneq (,$(wildcard ./.env))
    include .env
    export
endif

build-prod:
	docker buildx build --platform $(PROD_PLATFORM) \
		-t $(DOCKER_USERNAME)/$(IMAGE) --push .

UNIT_TESTS = $(patsubst tests/unit/%.rs,--test %,$(wildcard tests/unit/*.rs))

build:
	docker build --target runtime -t $(IMAGE) .

test:
	docker build --target test -t $(IMAGE)-test .
	docker run --rm $(IMAGE)-test cargo test $(UNIT_TESTS)

test-all: test

mutants:
	docker build --target mutants -t $(IMAGE)-mutants .
	docker run --rm $(IMAGE)-mutants \
		cargo mutants -j 3 --config config/mutants.toml -- $(UNIT_TESTS)

run:
	docker compose up --build -d
	docker compose logs -f

run-prod:
	RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) docker compose up --no-build -d
	docker compose logs -f

deploy:
	$(MAKE) test
	$(MAKE) build-prod
	scp Makefile docker-compose.yml pequenin:~/prices_comparer

.PHONY: build build-prod test test-all mutants run run-prod deploy
