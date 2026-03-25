IMAGE := voice-assistant-rust

build:
	docker build -t $(IMAGE) .

UID := $(shell id -u)

run:
	@./run.sh

test:
	docker run --rm -e SDL_AUDIODRIVER=dummy -e SDL_VIDEODRIVER=dummy $(IMAGE) \
		cargo test

debug:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		-v $(HOME)/.claude:/root/.claude \
		$(IMAGE) bash

.PHONY: build run test debug
