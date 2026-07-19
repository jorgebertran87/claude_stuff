IMAGE := fantastic-battle
FRONTEND_DIR := ../fantastic_battle_frontend
LOCAL_PLATFORM ?= linux/amd64

UNIT_TESTS = --lib $(patsubst tests/unit/%.rs,--test %,$(wildcard tests/unit/*.rs))
INTEGRATION_TESTS = $(patsubst tests/integration/%.rs,--test %,$(wildcard tests/integration/*.rs))

build-test:
	docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .

test: build-test
	docker run --rm $(IMAGE)-test cargo test $(UNIT_TESTS) $(INTEGRATION_TESTS)

build:
	docker build --platform $(LOCAL_PLATFORM) --target runtime -t $(IMAGE) .

run: build
	docker run --rm -p 8080:8080 $(IMAGE)

dev:
	RUST_LOG=info cargo run --bin fantastic_battle_server

up:
	docker compose up --build

down:
	docker compose down

dev-backend:
	RUST_LOG=info cargo run --bin fantastic_battle_server

dev-frontend:
	cd $(FRONTEND_DIR) && npx vite

test-all: test
	cd $(FRONTEND_DIR) && $(MAKE) test-all

.PHONY: build-test test build run dev up down dev-backend dev-frontend test-all
