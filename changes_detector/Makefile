IMAGE := changes-detector
SHELL := /bin/bash

ifneq (,$(wildcard ./.env))
    include .env
    export
endif

UNIT_TESTS        = $(patsubst tests/unit/%.rs,--test %,$(wildcard tests/unit/*.rs))
INTEGRATION_TESTS = $(patsubst tests/integration/%.rs,--test %,$(wildcard tests/integration/*.rs))

build:
	docker build --target runtime -t $(IMAGE) .

test:
	docker build --target test -t $(IMAGE)-test .
	docker run --rm $(IMAGE)-test cargo test $(UNIT_TESTS)

test-integration:
	docker build --target test -t $(IMAGE)-test .
	docker run --rm $(IMAGE)-test cargo test $(INTEGRATION_TESTS)

test-all:
	docker build --target test -t $(IMAGE)-test .
	docker run --rm $(IMAGE)-test cargo test $(UNIT_TESTS) $(INTEGRATION_TESTS)

run:
	docker compose up --build -d
	docker compose logs -f

.PHONY: build test test-integration test-all run
