IMAGE      := minesweeper-detector
IMAGES_DIR ?= $(HOME)

TEST_IMAGES := \
	telegram_image_1776362547800808255.jpg \
	photo_5985514867101731739_y.jpg

VOLS := $(foreach f,$(TEST_IMAGES),-v $(IMAGES_DIR)/$(f):/images/$(f):ro)

build:
	docker build -t $(IMAGE) .

test: build
	docker run --rm $(VOLS) $(IMAGE) pytest tests/ -v

parse: build
	docker run --rm \
		-v $(SCREENSHOT):/images/screenshot.jpg:ro \
		$(IMAGE) python main.py /images/screenshot.jpg

parse-json: build
	docker run --rm \
		-v $(SCREENSHOT):/images/screenshot.jpg:ro \
		$(IMAGE) python main.py /images/screenshot.jpg --json

.PHONY: build test parse parse-json
