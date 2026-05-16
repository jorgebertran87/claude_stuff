IMAGE   := flights-scanner
ENVFILE := .env

.PHONY: help test build run

help:
	@echo "Usage: make <target>"
	@echo ""
	@echo "  test   Run all tests and clippy inside Docker"
	@echo "  build  Build the release Docker image"
	@echo "  run    Build and start the server on port 3000"
	@echo ""
	@echo "Copy .env.example to .env and fill in your API key before running."

test:
	docker build --target tester -t $(IMAGE)-test .

build:
	docker build -t $(IMAGE) .

$(ENVFILE):
	cp .env.example $@
	@echo "Created $(ENVFILE) from .env.example — fill in your API key and re-run."

run: build $(ENVFILE)
	docker run --rm -p 3000:3000 --env-file $(ENVFILE) $(IMAGE)
