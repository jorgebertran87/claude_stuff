IMAGE   := minesweeper-detector
IMG     ?= /images/screenshot.jpg

build:
	docker build -t $(IMAGE) .

test: build
	docker run --rm \
		-v $(SCREENSHOT):/images/screenshot.jpg:ro \
		$(IMAGE) pytest tests/ -v

parse: build
	docker run --rm \
		-v $(SCREENSHOT):/images/screenshot.jpg:ro \
		$(IMAGE) python main.py $(IMG)

parse-json: build
	docker run --rm \
		-v $(SCREENSHOT):/images/screenshot.jpg:ro \
		$(IMAGE) python main.py $(IMG) --json

.PHONY: build test parse parse-json
