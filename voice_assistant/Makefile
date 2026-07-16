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

# FILE must point to the infrastructure source file (e.g. src/infrastructure/transcriber/google_stt.rs).
# The test name is derived from the parent directory (the port name).
FILE_TEST = $(if $(FILE),--test $(notdir $(patsubst %/,%,$(dir $(FILE)))),$(INTEGRATION_TESTS))

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
		--device /dev/snd \
		--env-file .env \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		-v $(HOME)/.claude:/root/.claude \
		-v $(PWD)/.google_refresh_token:/app/.google_refresh_token \
		$(IMAGE)-mutants \
		cargo mutants -j 3 --config config/mutants-integration.toml -f $(FILE) -- $(FILE_TEST)

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
	scp -r searxng pequenin:~/pequenin/
	ssh pequenin 'cd ~/pequenin && docker compose down --rmi all'

connect-speaker:
	ssh pequenin '\
		bluetoothctl power on >/dev/null && \
		{ bluetoothctl info 81:8B:AE:38:BD:5C | grep -q "Paired: yes" || \
		  { echo "Pairing speaker..."; bluetoothctl remove 81:8B:AE:38:BD:5C 2>/dev/null; sleep 1; \
		    bluetoothctl scan on & sleep 3; bluetoothctl pair 81:8B:AE:38:BD:5C; bluetoothctl scan off; }; } && \
		bluetoothctl trust 81:8B:AE:38:BD:5C && \
		bluetoothctl connect 81:8B:AE:38:BD:5C && \
		sleep 2 && \
		pactl set-card-profile $$(pactl list short cards | grep -i bluez | head -1 | awk "{print $$1}") a2dp_sink 2>/dev/null; \
		SINK=$$(pactl list short sinks | grep -i bluez | head -1 | awk "{print $$1}"); \
		[ -n "$$SINK" ] && pactl set-default-sink "$$SINK" && echo "Speaker connected: $$SINK" || echo "ERROR: no Bluetooth sink found"'

.PHONY: build build-local run run-prod run-telegram run-telegram-prod test test-integration mutants mutants-integration debug deploy connect-speaker
