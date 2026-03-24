IMAGE := voice-listener

build:
	docker build -t $(IMAGE) .

UID := $(shell id -u)

run:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-e SDL_AUDIODRIVER=pulse \
		-e SDL_VIDEODRIVER=dummy \
		--env-file .env \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		-v $(HOME)/.claude:/root/.claude \
		-v $(HOME)/.claude.json:/root/.claude.json \
		$(IMAGE)

test:
	docker run --rm $(IMAGE) pytest tests/ -v

debug:
	docker run -it --rm \
		-e PULSE_SERVER=unix:/tmp/pulse.sock \
		-v /run/user/$(UID)/pulse/native:/tmp/pulse.sock \
		-v $(HOME)/.claude:/root/.claude \
		$(IMAGE) python debug_audio.py

.PHONY: build run test debug
