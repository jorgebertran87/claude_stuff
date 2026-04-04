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

test tests:
	rsync -a --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target builder -t $(IMAGE)-builder .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-builder \
		cargo test $(UNIT_TESTS)

test-integration:
	rsync -a --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target builder -t $(IMAGE)-builder .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-builder \
		cargo test --test tts_pipeline --test tts_http_integration --test speech_recognition_integration --test claude_cli_integration

mutants:
	rsync -a --delete ../features/ features/
	docker build --platform $(LOCAL_PLATFORM) --target builder -t $(IMAGE)-builder .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-builder \
		cargo mutants -- $(UNIT_TESTS)

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

.PHONY: build build-local run run-prod run-telegram run-telegram-prod test tests test-integration mutants debug deploy
