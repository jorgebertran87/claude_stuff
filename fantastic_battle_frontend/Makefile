IMAGE := fantastic-battle-frontend
LOCAL_PLATFORM ?= linux/amd64

dev:
	npx vite

build:
	npx tsc --noEmit && npx vite build

test: build
	npx vite &
	sleep 3
	npx cucumber-js --require-module tsx/esm
	kill %1 2>/dev/null || true

.PHONY: dev build test
