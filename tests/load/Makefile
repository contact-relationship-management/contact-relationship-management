# Load tests (k6) against the CRM API: smoke, basic, stress, spike and soak.
#
# Needs a running service at TEST_URL with a throwaway database: the tests write
# synthetic rows and do not delete them. From the repository root,
# `podman compose up --build` starts one at http://localhost:1241.
#
#   make smoke                     # one user, each call once; run this first
#   make basic                     # 10 users for 5 minutes
#   make stress                    # steps of 1x, 2x and 4x VUS
#   make spike                     # VUS, then 10x VUS for DURATION, then back
#   make soak                      # 10 users for 2 hours
#   make seed                      # SEED synthetic contacts (default 1000), and records around them
#   make endpoints                 # each group of routes on its own at RATE a second (default 20)
#   make breakpoint                # a rising rate up to MAX_RATE (default 200) until it fails
#   make bench                     # seed, then endpoints: the figures for BENCHMARKS.md
#   make quick                     # smoke, then one minute of basic
#   make basic VUS=20 DURATION=10m
#   make smoke K6=podman-k6        # k6 from a container, no local install
#
# VUS and DURATION are passed only when set; each script has its own defaults.
# Results are written to results/: a JSON summary and k6's HTML report per test.

TEST_URL ?= http://localhost:1241
AUTH_TOKEN ?=
VUS ?=
DURATION ?=
SEED ?=
RATE ?=
MAX_RATE ?=

# k6 from the host. Set K6=podman-k6 to run the pinned image with Podman
# instead; inside the container the host's loopback is host.containers.internal.
K6 ?= k6
K6_IMAGE ?= docker.io/grafana/k6:1.4.2@sha256:3656673de3f30424e8ebcfa46acd9558d83b6a43612d0f668ffeac953950c6c7

RESULTS_DIR := results

ifeq ($(K6),podman-k6)
K6_RUN = podman run --rm -i -v "$(CURDIR):/work:Z" -w /work -e K6_WEB_DASHBOARD -e K6_WEB_DASHBOARD_EXPORT $(K6_IMAGE) run
K6_URL = $(subst localhost,host.containers.internal,$(subst 127.0.0.1,host.containers.internal,$(TEST_URL)))
else
K6_RUN = $(K6) run
K6_URL = $(TEST_URL)
endif

K6_ENV = -e TEST_URL=$(K6_URL) -e CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_TOKEN=$(AUTH_TOKEN) \
	$(if $(VUS),-e VUS=$(VUS)) $(if $(DURATION),-e DURATION=$(DURATION)) \
	$(if $(SEED),-e SEED=$(SEED)) $(if $(RATE),-e RATE=$(RATE)) $(if $(MAX_RATE),-e MAX_RATE=$(MAX_RATE))

# $(call run,<script name>): the summary and the HTML report go to results/.
define run
	@mkdir -p $(RESULTS_DIR)
	K6_WEB_DASHBOARD=true K6_WEB_DASHBOARD_EXPORT=$(RESULTS_DIR)/$(1).html \
		$(K6_RUN) $(K6_ENV) --summary-export $(RESULTS_DIR)/$(1)-summary.json scripts/$(1).test.js
endef

.PHONY: smoke basic stress spike soak seed endpoints breakpoint bench quick clean help

smoke:
	$(call run,smoke)

basic:
	$(call run,basic-load)

stress:
	$(call run,stress)

spike:
	$(call run,spike)

soak:
	$(call run,soak)

seed:
	$(call run,seed)

endpoints:
	$(call run,endpoints)

breakpoint:
	$(call run,breakpoint)

bench: seed endpoints

quick:
	$(call run,smoke)
	$(MAKE) basic DURATION=1m

clean:
	rm -rf $(RESULTS_DIR)

help:
	@sed -n '1,21p' Makefile
