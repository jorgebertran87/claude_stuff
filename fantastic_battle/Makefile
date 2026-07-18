IMAGE := fantastic-battle
LOCAL_PLATFORM ?= linux/amd64

UNIT_TESTS = --lib $(patsubst tests/unit/%.rs,--test %,$(wildcard tests/unit/*.rs))

build-test:
	docker build --platform $(LOCAL_PLATFORM) -t $(IMAGE)-test .

test: build-test
	docker run --rm $(IMAGE)-test cargo test $(UNIT_TESTS)

.PHONY: build-test test
