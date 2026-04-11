IMAGE := voice-assistant-rust
SHELL := /bin/bash

ifneq (,$(wildcard ./.env))
    include .env
    export
endif

build-prod:
	docker buildx build --platform $(PROD_PLATFORM) \
		-t $(DOCKER_USERNAME)/$(IMAGE) --push .

build:
	docker build --platform $(LOCAL_PLATFORM) -t $(IMAGE) .

UID := $(shell id -u)

run:
	@./run.sh $(if $(ORDER),--order "$(ORDER)") &

run-prod:
	@RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) ./run.sh $(if $(ORDER),--order "$(ORDER)") &

run-telegram:
	@./run.sh --telegram &

run-telegram-prod:
	@RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) ./run.sh --telegram &

run-both:
	@./run.sh --both &

run-both-prod:
	@RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) ./run.sh --both &

UNIT_TESTS = --lib \
	--test wake_word_detection \
	--test order_capture \
	--test interruptible_speech \
	--test audio_processing \
	--test conversation_flow \
	--test claude_handler_token_logging \
	--test session_continuity \
	--test skill_routing \
	--test cli_direct_order

INTEGRATION_TESTS = \
	--test speaker \
	--test transcriber \
	--test claude_handler \
	--test google_sheets \
	--test audio \
	--test speech \
	--test telegram_bot

# Map infrastructure source file → relevant integration test (name matches file stem)
file_tests = $(if $(FILE),--test $(basename $(notdir $(FILE))),$(INTEGRATION_TESTS))

test:
	docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-test \
		cargo test $(UNIT_TESTS)

test-integration:
	touch .google_refresh_token
	docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker compose -f config/docker-compose.test.yml run --rm integration-tests cargo test $(INTEGRATION_TESTS)

mutants:
	rsync -a --checksum --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target mutants -t $(IMAGE)-mutants .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-mutants \
		cargo mutants -- $(UNIT_TESTS)

mutants-integration:
	rsync -a --checksum --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target mutants -t $(IMAGE)-mutants .
	docker run --rm \
		--network host \
		--env-file .env \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		-v $(HOME)/.claude:/root/.claude \
		$(IMAGE)-mutants \
		cargo mutants -j 3 --config config/mutants-integration.toml $(if $(FILE),-f $(FILE)) -- $(file_tests)

debug:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		-v $(HOME)/.claude:/root/.claude \
		$(DOCKER_USERNAME)/$(IMAGE) bash

deploy:
	$(MAKE) build-prod
	scp Makefile run.sh pequenin:~/pequenin	
	scp -r .claude pequenin:~/pequenin/.claude

.PHONY: build build-local run run-prod run-telegram run-telegram-prod run-both run-both-prod test test-integration mutants mutants-integration debug deploy
