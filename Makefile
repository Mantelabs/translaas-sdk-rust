.PHONY: help lint test coverage fmt fmt-check clippy build check clean

help:
	just help

lint:
	just lint

test:
	just test

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
