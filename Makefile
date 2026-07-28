.PHONY: help lint test test-integration coverage fmt fmt-check clippy build check clean

help:
	just help

lint:
	just lint

test:
	just test

test-integration:
	just test-integration

coverage:
	just coverage

fmt:
	just fmt

fmt-check:
	just fmt-check

clippy:
	just clippy

build:
	just build

check:
	just check

clean:
	just clean
