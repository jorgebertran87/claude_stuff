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
	--test speaker_integration \
	--test speech_recognition_integration \
	--test claude_cli_integration \
	--test google_sheets_integration

# Map infrastructure source file → relevant integration test(s)
file_tests = $(if $(FILE),$(or \
    $(if $(findstring google_sheets,$(FILE)),--test google_sheets_integration),\
    $(if $(findstring speaker,$(FILE)),--test speaker_integration),\
    $(if $(findstring transcriber,$(FILE)),--test speech_recognition_integration),\
    $(if $(findstring claude_handler,$(FILE)),--test claude_cli_integration),\
    $(INTEGRATION_TESTS)),$(INTEGRATION_TESTS))

test tests:
	rsync -a --checksum --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-test \
		cargo test $(UNIT_TESTS)

test-integration:
	rsync -a --checksum --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker compose -f docker-compose.test.yml run --rm integration-tests cargo test $(INTEGRATION_TESTS)

mutants:
	rsync -a --checksum --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-test \
		cargo mutants -- $(UNIT_TESTS)

mutants-integration:
	rsync -a --checksum --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target test -t $(IMAGE)-test .
	docker run --rm \
		--network host \
		--env-file .env \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		-v $(HOME)/.claude:/root/.claude \
		$(IMAGE)-test \
		cargo mutants -j 3 --config .cargo/mutants-integration.toml $(if $(FILE),-f $(FILE)) -- $(file_tests)

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

.PHONY: build build-local run run-prod run-telegram run-telegram-prod test tests test-integration mutants mutants-integration debug deploy
