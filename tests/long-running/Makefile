# Long-running tests (k6).
#
# Needs a running service at TEST_URL with a throwaway database: the tests
# write synthetic rows and do not delete them. From the repository root,
# `podman compose up --build` starts one at http://localhost:1241.
#
#   make stability                 # 6 hours, 5 VUs
#   make quick                     # 10 minutes, for checking the script
#   make stability DURATION=24h VUS=10
#   make stability K6=podman-k6    # k6 from a container, no local install
#
# Results are written to results/ as JSON summaries.

TEST_URL ?= http://localhost:1241
AUTH_TOKEN ?=
VUS ?= 5
DURATION ?= 6h
RAMP ?= 5m

# k6 from the host. Set K6=podman-k6 to run the pinned image with Podman
# instead; inside the container the host's loopback is host.containers.internal.
K6 ?= k6
K6_IMAGE ?= docker.io/grafana/k6:1.4.2@sha256:3656673de3f30424e8ebcfa46acd9558d83b6a43612d0f668ffeac953950c6c7

RESULTS_DIR := results

ifeq ($(K6),podman-k6)
K6_RUN = podman run --rm -i -v "$(CURDIR):/work:Z" -w /work $(K6_IMAGE) run
K6_URL = $(subst localhost,host.containers.internal,$(subst 127.0.0.1,host.containers.internal,$(TEST_URL)))
else
K6_RUN = $(K6) run
K6_URL = $(TEST_URL)
endif

K6_ENV = -e TEST_URL=$(K6_URL) -e CRM_AUTH_TOKEN=$(AUTH_TOKEN) -e VUS=$(VUS)

.PHONY: stability quick clean help

stability:
	@mkdir -p $(RESULTS_DIR)
	$(K6_RUN) $(K6_ENV) -e DURATION=$(DURATION) -e RAMP=$(RAMP) \
		--summary-export $(RESULTS_DIR)/stability-summary.json scripts/stability.test.js

quick:
	@mkdir -p $(RESULTS_DIR)
	$(K6_RUN) $(K6_ENV) -e DURATION=10m -e RAMP=1m \
		--summary-export $(RESULTS_DIR)/quick-summary.json scripts/stability.test.js

clean:
	rm -rf $(RESULTS_DIR)

help:
	@sed -n '1,13p' Makefile
