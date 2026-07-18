IMAGE := fantastic-battle-frontend
LOCAL_PLATFORM ?= linux/amd64

dev:
	npx vite

build:
	npx tsc --noEmit && npx vite build

test: build
	@for port in $$(seq 5173 5180); do kill $$(lsof -ti:$$port) 2>/dev/null || true; done
	@npx vite --port 5173 &
	@sleep 3
	npx cucumber-js --require-module tsx/esm; EXIT=$$?; kill %1 2>/dev/null || true; exit $$EXIT

.PHONY: dev build test
