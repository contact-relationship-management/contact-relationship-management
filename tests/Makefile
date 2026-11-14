# Tests that need a running service: load tests (load/) and long-running tests (long-running/).
#
#   make load-smoke        load smoke test
#   make load-quick        load smoke, then one minute of basic load
#   make long-quick        10-minute stability run
#   make long-stability    6-hour stability run
#   make clean             remove test artifacts
#
# Start a service first, for example `podman compose up --build` from the repository root.
# The variables below are passed through to each directory's Makefile.

TEST_URL ?= http://localhost:1241
AUTH_TOKEN ?=

PASS = TEST_URL=$(TEST_URL) AUTH_TOKEN=$(AUTH_TOKEN)

.PHONY: load-smoke load-quick long-quick long-stability clean help

load-smoke:
	$(MAKE) -C load smoke $(PASS)

load-quick:
	$(MAKE) -C load quick $(PASS)

long-quick:
	$(MAKE) -C long-running quick $(PASS)

long-stability:
	$(MAKE) -C long-running stability $(PASS)

clean:
	$(MAKE) -C load clean
	$(MAKE) -C long-running clean

help:
	@sed -n '1,10p' Makefile
