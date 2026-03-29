IMAGE := voice-assistant-rust
SHELL := /bin/bash

ifneq (,$(wildcard ./.env))
    include .env
    export
endif

build-prod:
	docker buildx build --platform linux/amd64,linux/arm64 \
		-t $(DOCKER_USERNAME)/$(IMAGE) --push .

build:
	docker build --platform linux/amd64 -t $(IMAGE) .

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

deploy:
	scp Makefile run.sh pequenin:~/pequenin		

.PHONY: build build-local run run-prod run-telegram run-telegram-prod test debug push
