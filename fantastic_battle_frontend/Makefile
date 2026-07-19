IMAGE := fantastic-battle-frontend
BACKEND_IMAGE := fantastic-battle-test
BACKEND_CONTAINER := fantastic-battle-server
LOCAL_PLATFORM ?= linux/amd64

dev:
	npx vite

build:
	npx tsc --noEmit && npx vite build

docker-build:
	docker build --platform $(LOCAL_PLATFORM) -t $(IMAGE) .

docker-run: docker-build
	docker run --rm -p 80:80 $(IMAGE)

start-backend:
	@if ! docker ps --format '{{.Names}}' | grep -q '^$(BACKEND_CONTAINER)$$'; then \
		echo "Starting backend server..."; \
		docker run -d --rm --name $(BACKEND_CONTAINER) \
			-p 127.0.0.1:8081:8080 \
			$(BACKEND_IMAGE) \
			cargo run --bin fantastic_battle_server; \
		sleep 5; \
	fi

stop-backend:
	docker kill $(BACKEND_CONTAINER) 2>/dev/null || true

test: build
	@for port in $$(seq 5173 5180); do kill $$(lsof -ti:$$port) 2>/dev/null || true; done
	@npx vite --port 5173 &
	@sleep 3
	npx cucumber-js --require-module tsx/esm --tags "not @api"; EXIT=$$?; kill %1 2>/dev/null || true; exit $$EXIT

test-api: build start-backend
	@for port in $$(seq 5173 5180); do kill $$(lsof -ti:$$port) 2>/dev/null || true; done
	@npx vite --port 5173 &
	@sleep 3
	npx cucumber-js --require-module tsx/esm --tags @api; EXIT=$$?; kill %1 2>/dev/null || true; $(MAKE) stop-backend; exit $$EXIT

test-all: build start-backend
	@for port in $$(seq 5173 5180); do kill $$(lsof -ti:$$port) 2>/dev/null || true; done
	@npx vite --port 5173 &
	@sleep 3
	npx cucumber-js --require-module tsx/esm; EXIT=$$?; kill %1 2>/dev/null || true; $(MAKE) stop-backend; exit $$EXIT

.PHONY: dev build docker-build docker-run test test-api test-all start-backend stop-backend
