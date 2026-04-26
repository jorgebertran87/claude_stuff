IMAGE      := minesweeper-detector
IMAGES_DIR ?= $(CURDIR)/tests/images

TEST_IMAGES := \
	telegram_image_1776362547800808255.jpg \
	photo_5985514867101731739_y.jpg \
	photo_5983263067288046793_y.jpg \
	photo_5987560487010372572_y.jpg \
	photo_5999345726946741438_y.jpg \
	photo_6012830459672333346_w.jpg \
	photo_6012830459672333473_w.jpg

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
