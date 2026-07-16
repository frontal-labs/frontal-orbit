# Frontal Orbit — Bazel monorepo Makefile.
# Every target delegates to scripts/<x>.sh so behavior is identical locally
# and in CI. Pass extra args via `make <target> ARGS="..."`.

BAZEL ?= bazel
ARGS  ?=

.PHONY: build test lint fmt tidy bootstrap doctor clean coverage ci cache workspace version bench telemetry codegen generators remote fuzz cli changeset renovate

build:
	./scripts/build.sh $(ARGS)

test:
	./scripts/test.sh $(ARGS)

lint:
	./scripts/lint.sh $(ARGS)

fmt:
	./scripts/fmt.sh $(ARGS)

tidy:
	./scripts/tidy.sh $(ARGS)

bootstrap:
	./scripts/bootstrap.sh $(ARGS)

doctor:
	./scripts/doctor.sh $(ARGS)

clean:
	./scripts/clean.sh $(ARGS)

coverage:
	./scripts/coverage.sh $(ARGS)

ci:
	./scripts/ci.sh $(ARGS)

# --- Dev tools (canonical implementations live under //tools) --------------
cache:
	./scripts/cache.sh $(ARGS)

workspace:
	./scripts/workspace.sh $(ARGS)

version:
	./scripts/version.sh $(ARGS)

bench:
	./scripts/bench.sh $(ARGS)

telemetry:
	./scripts/telemetry.sh $(ARGS)

codegen:
	./scripts/codegen.sh $(ARGS)

generators:
	./scripts/generators.sh $(ARGS)

remote:
	./scripts/remote.sh $(ARGS)

fuzz:
	./scripts/fuzz.sh $(ARGS)

cli:
	./scripts/cli.sh $(ARGS)

# --- Dependency & release automation --------------------------------------
changeset:
	./scripts/changeset.sh $(ARGS)

renovate:
	./scripts/renovate.sh $(ARGS)
