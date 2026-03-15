IMAGE_NAME = claude_tdd_go

.PHONY: test build

build:
	docker build -t $(IMAGE_NAME) .

test: build
	docker run --rm $(IMAGE_NAME)
