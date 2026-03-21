IMAGE := voice-listener

build:
	docker build -t $(IMAGE) .

UID := $(shell id -u)
GID := $(shell id -g)

run:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		$(IMAGE)

debug:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		$(IMAGE) python debug_audio.py

.PHONY: build run debug install run-local
