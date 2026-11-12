# Makefile for Load Testing
#
# This Makefile provides targets for running various load tests.
#
# Prerequisites:
# - k6 installed (https://k6.io/docs/get-started/installation/)
# - Application running at TEST_URL (default: http://localhost:1241)
# - Optional: AUTH_TOKEN for authenticated tests
#
# Usage:
#   make smoke              # Run smoke test
#   make basic              # Run basic load test
#   make stress             # Run stress test
#   make soak               # Run soak (long-running) test
#   make spike              # Run spike test
#   make api-contract      # Run API contract test
#   make database           # Run database test
#   make bridges            # Run bridges integration test
#   make all                # Run all tests
#   make clean              # Clean up test artifacts
#
# Configuration:
#   TEST_URL=http://localhost:1241 make smoke
#   AUTH_TOKEN=your-token make basic
#   VUS=20 DURATION=5m make stress

# Configuration variables
TEST_URL ?= http://localhost:1241
AUTH_TOKEN ?= 
REQUIRE_AUTH ?= false
VUS ?= 10
DURATION ?= 1m
ENV ?= dev

# k6 executable
K6 ?= k6

# Directories
SCRIPTS_DIR ?= scripts
CONFIG_DIR ?= config
LIB_DIR ?= lib
OUTPUT_DIR ?= output
RESULTS_DIR ?= results

# Ensure output directories exist
$(shell mkdir -p $(OUTPUT_DIR) $(RESULTS_DIR))

# Common k6 flags
K6_FLAGS = --env TEST_URL=$(TEST_URL) \
	--env CRM_AUTH_TOKEN=$(AUTH_TOKEN) \
	--env REQUIRE_AUTH=$(REQUIRE_AUTH) \
	--env ENV=$(ENV)

# Target: Run smoke test
.PHONY: smoke
smoke:
	@echo "Running smoke test..."
	@echo "  URL: $(TEST_URL)"
	@echo "  Environment: $(ENV)"
	@echo ""
	$(K6) run $(K6_FLAGS) --vus 5 --duration 1m $(SCRIPTS_DIR)/smoke.test.js

# Target: Run basic load test
.PHONY: basic
basic:
	@echo "Running basic load test..."
	@echo "  URL: $(TEST_URL)"
	@echo "  VUs: $(VUS)"
	@echo "  Duration: $(DURATION)"
	@echo "  Environment: $(ENV)"
	@echo ""
	$(K6) run $(K6_FLAGS) --vus $(VUS) --duration $(DURATION) $(SCRIPTS_DIR)/basic-load.test.js

# Target: Run stress test
.PHONY: stress
stress:
	@echo "Running stress test..."
	@echo "  URL: $(TEST_URL)"
	@echo "  VUs: $(shell echo $$(( $(VUS) * 4 )))"
	@echo "  Duration: $(DURATION)"
	@echo "  Environment: $(ENV)"
	@echo ""
	$(K6) run $(K6_FLAGS) --vus $(shell echo $$(( $(VUS) * 4 ))) --duration $(DURATION) $(SCRIPTS_DIR)/stress.test.js

# Target: Run soak test (long-running)
.PHONY: soak
soak:
	@echo "Running soak test (long-running)..."
	@echo "  URL: $(TEST_URL)"
	@echo "  VUs: $(VUS)"
	@echo "  Duration: 6h"
	@echo "  Environment: $(ENV)"
	@echo ""
	@echo "WARNING: This test will run for 6 hours!"
	@echo "Use Ctrl+C to stop early if needed."
	@echo ""
	$(K6) run $(K6_FLAGS) --vus $(VUS) --duration 6h $(SCRIPTS_DIR)/soak.test.js

# Target: Run spike test
.PHONY: spike
spike:
	@echo "Running spike test..."
	@echo "  URL: $(TEST_URL)"
	@echo "  VUs: $(VUS)"
	@echo "  Duration: 2m"
	@echo "  Environment: $(ENV)"
	@echo ""
	$(K6) run $(K6_FLAGS) --vus $(VUS) --duration 2m $(SCRIPTS_DIR)/spike.test.js

# Target: Run API contract test
.PHONY: api-contract
api-contract:
	@echo "Running API contract test..."
	@echo "  URL: $(TEST_URL)"
	@echo "  VUs: 5"
	@echo "  Duration: 2m"
	@echo "  Environment: $(ENV)"
	@echo ""
	$(K6) run $(K6_FLAGS) --vus 5 --duration 2m $(SCRIPTS_DIR)/api-contract.test.js

# Target: Run database test
.PHONY: database
database:
	@echo "Running database test..."
	@echo "  URL: $(TEST_URL)"
	@echo "  VUs: $(VUS)"
	@echo "  Duration: $(DURATION)"
	@echo "  Environment: $(ENV)"
	@echo ""
	$(K6) run $(K6_FLAGS) --vus $(VUS) --duration $(DURATION) $(SCRIPTS_DIR)/database.test.js

# Target: Run bridges integration test
.PHONY: bridges
bridges:
	@echo "Running bridges integration test..."
	@echo "  URL: $(TEST_URL)"
	@echo "  VUs: 5"
	@echo "  Duration: $(DURATION)"
	@echo "  Environment: $(ENV)"
	@echo ""
	$(K6) run $(K6_FLAGS) --vus 5 --duration $(DURATION) $(SCRIPTS_DIR)/bridges.test.js

# Target: Run all tests
.PHONY: all
all: smoke basic stress api-contract database bridges

# Target: Run all quick tests (no long-running)
.PHONY: quick
quick: smoke basic stress spike api-contract database bridges

# Target: Run with InfluxDB output
.PHONY: with-influxdb
with-influxdb:
	@echo "Running tests with InfluxDB output..."
	@echo "Configure INFLUXDB_URL, INFLUXDB_TOKEN, INFLUXDB_ORG, INFLUXDB_BUCKET"
	$(K6) run $(K6_FLAGS) --out influxdb=http://localhost:8086/k6 $(SCRIPTS_DIR)/smoke.test.js

# Target: Run with Prometheus output
.PHONY: with-prometheus
with-prometheus:
	@echo "Running tests with Prometheus output..."
	$(K6) run $(K6_FLAGS) --out experimental-prometheus-rw $(SCRIPTS_DIR)/smoke.test.js

# Target: Run with JSON output
.PHONY: with-json
with-json:
	@echo "Running tests with JSON output..."
	$(K6) run $(K6_FLAGS) --out json=$(OUTPUT_DIR)/results.json $(SCRIPTS_DIR)/smoke.test.js

# Target: Run with cloud output
.PHONY: with-cloud
with-cloud:
	@echo "Running tests with k6 Cloud output..."
	@echo "Set K6_CLOUD_TOKEN environment variable"
	$(K6) cloud $(SCRIPTS_DIR)/smoke.test.js

# Target: Clean up test artifacts
.PHONY: clean
clean:
	@echo "Cleaning up test artifacts..."
	rm -rf $(OUTPUT_DIR)/*.json
	rm -rf $(OUTPUT_DIR)/*.log
	rm -rf $(RESULTS_DIR)/*.json
	rm -rf $(RESULTS_DIR)/*.html
	rm -rf $(RESULTS_DIR)/*.png

# Target: Show help
.PHONY: help
help:
	@echo "Load Testing Makefile"
	@echo ""
	@echo "Available targets:"
	@echo ""
	@echo "  smoke              Run smoke test (1 minute)"
	@echo "  basic              Run basic load test"
	@echo "  stress             Run stress test (4x VUs)"
	@echo "  soak               Run soak test (6 hours, long-running)"
	@echo "  spike              Run spike test (sudden traffic surge)"
	@echo "  api-contract      Run API contract compliance test"
	@echo "  database           Run database performance test"
	@echo "  bridges            Run bridges integration test"
	@echo "  all                Run all tests"
	@echo "  quick              Run all quick tests (no long-running)"
	@echo ""
	@echo "With outputs:"
	@echo "  with-influxdb     Run with InfluxDB output"
	@echo "  with-prometheus   Run with Prometheus output"
	@echo "  with-json         Run with JSON output"
	@echo "  with-cloud        Run with k6 Cloud output"
	@echo ""
	@echo "Configuration:"
	@echo "  TEST_URL=http://localhost:1241 make smoke"
	@echo "  AUTH_TOKEN=your-token make basic"
	@echo "  VUS=20 DURATION=5m make stress"
	@echo ""
	@echo "Utility:"
	@echo "  clean              Clean up test artifacts"
	@echo "  help               Show this help message"
	@echo ""

# Target: Validate configuration
.PHONY: validate
validate:
	@echo "Validating configuration..."
	@if [ -z "$(TEST_URL)" ]; then \
		echo "WARNING: TEST_URL is not set, using default: http://localhost:1241"; \
	fi
	@if [ -z "$(AUTH_TOKEN)" ] && [ "$(REQUIRE_AUTH)" = "true" ]; then \
		echo "WARNING: REQUIRE_AUTH is true but AUTH_TOKEN is not set"; \
	fi
	@echo "Configuration looks good!"

# Target: Install dependencies
.PHONY: install
install:
	@echo "Installing k6..."
	@echo "Follow instructions at: https://k6.io/docs/get-started/installation/"
