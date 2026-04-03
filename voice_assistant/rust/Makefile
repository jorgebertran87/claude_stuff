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
	docker build --platform $(LOCAL_PLATFORM) --target builder -t $(IMAGE)-builder .
	docker run --rm \
		-e SDL_AUDIODRIVER=dummy \
		-e SDL_VIDEODRIVER=dummy \
		$(IMAGE)-builder \
		cargo test

debug:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		-v $(HOME)/.claude:/root/.claude \
		$(DOCKER_USERNAME)/$(IMAGE) bash

deploy:
	scp Makefile run.sh pequenin:~/pequenin		

.PHONY: build build-local run run-prod run-telegram run-telegram-prod test debug push deploy
