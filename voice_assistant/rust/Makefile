IMAGE := voice-assistant-rust
SHELL := /bin/bash

ifneq (,$(wildcard ./.env))
    include .env
    export
endif

build:
	docker build -t $(DOCKER_USERNAME)/$(IMAGE) .

push:
	docker push $(DOCKER_USERNAME)/$(IMAGE)

UID := $(shell id -u)

run:
	@./run.sh $(if $(ORDER),--order "$(ORDER)") &

run-prod:
	@RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) ./run.sh $(if $(ORDER),--order "$(ORDER)") &

run-telegram:
	@./run.sh --telegram &

run-telegram-prod:
	@RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) ./run.sh --telegram &

test:
	docker run --rm -e SDL_AUDIODRIVER=dummy -e SDL_VIDEODRIVER=dummy $(DOCKER_USERNAME)/$(IMAGE) \
		cargo test

debug:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		-v $(HOME)/.claude:/root/.claude \
		$(DOCKER_USERNAME)/$(IMAGE) bash

.PHONY: build run run-prod run-telegram run-telegram-prod test debug push
