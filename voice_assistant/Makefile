IMAGE := voice-assistant-rust
SHELL := /bin/bash

ifneq (,$(wildcard ./.env))
    include .env
    export
endif

MINESWEEPER_IMAGE := minesweeper-detector

# Ensure the shared crates are available inside the Docker build context.
_shared = rsync -a --delete --exclude=target ../deepseek_client/ ./deepseek_client/ && \
          rsync -a --delete --exclude=target ../skill_loader/    ./skill_loader/    &&

build-prod:
	$(_shared) docker buildx build --platform $(PROD_PLATFORM) \
		-t $(DOCKER_USERNAME)/$(IMAGE) --push .
	docker buildx build --platform $(PROD_PLATFORM) \
		-t $(DOCKER_USERNAME)/$(MINESWEEPER_IMAGE) --push ../minesweeper

build:
	$(_shared) docker build --platform $(LOCAL_PLATFORM) -t $(IMAGE) .

UID := $(shell id -u)

run:
	@./run.sh $(if $(ORDER),--order "$(ORDER)") &

run-prod:
	@RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) ./run.sh $(if $(ORDER),--order "$(ORDER)") &

run-telegram:
	UID=$(UID) docker compose up --build -d
	docker compose logs -f

run-telegram-prod:
	UID=$(UID) RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) MINESWEEPER_IMAGE=$(DOCKER_USERNAME)/$(MINESWEEPER_IMAGE) \
		docker compose up --no-build -d
	docker compose logs -f


UNIT_TESTS = --lib $(patsubst tests/unit/%.rs,--test %,$(wildcard tests/unit/*.rs))
INTEGRATION_TESTS = $(patsubst tests/integration/%.rs,--test %,$(wildcard tests/integration/*.rs))

# Map infrastructure source file → relevant integration test (name matches file stem)
FILE_TESTS = $(if $(FILE),--test $(basename $(notdir $(FILE))),$(INTEGRATION_TESTS))

test:
	$(_shared) docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-test \
		cargo test $(UNIT_TESTS)

test-integration:
	touch .google_refresh_token
	$(_shared) docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker compose -f config/docker-compose.test.yml run --rm integration-tests cargo test $(INTEGRATION_TESTS)

mutants:
	$(_shared) docker build --platform $(LOCAL_PLATFORM) --target mutants -t $(IMAGE)-mutants .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-mutants \
		cargo mutants -j 3 --config config/mutants.toml -- $(UNIT_TESTS)

mutants-integration:
	touch .google_refresh_token
	$(_shared) docker build --platform $(LOCAL_PLATFORM) --target mutants -t $(IMAGE)-mutants .
	docker run --rm \
		--network host \
		--env-file .env \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		-v $(HOME)/.claude:/root/.claude \
		-v $(PWD)/.google_refresh_token:/app/.google_refresh_token \
		$(IMAGE)-mutants \
		cargo mutants -j 3 --config config/mutants-integration.toml $(if $(FILE),-f $(FILE)) -- $(FILE_TESTS)

debug:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		-v $(HOME)/.claude:/root/.claude \
		$(DOCKER_USERNAME)/$(IMAGE) bash

deploy:
	$(MAKE) test
	$(MAKE) test-integration
	$(MAKE) build-prod
	scp Makefile run.sh docker-compose.yml pequenin:~/pequenin
	scp -r .claude pequenin:~/pequenin/

.PHONY: build build-local run run-prod run-telegram run-telegram-prod test test-integration mutants mutants-integration debug deploy
