IMAGE_NAME = claude_tdd_ts

.PHONY: test build

build:
	docker build -t $(IMAGE_NAME) .

test: build
	docker run --rm $(IMAGE_NAME)
