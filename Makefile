# Makefile for contact-relationship-management
# Root Makefile that starts both service and UI in demo mode

.PHONY: demo
demo:
	@echo "Starting CRM service with migrations and seed data..."
	cd contact-relationship-management-service-with-rust && make demo &
	SERVICE_PID=$$!
	@echo "Service PID: $$SERVICE_PID"
	@echo "Waiting for service to start..."
	sleep 5
	@echo "Starting CRM UI..."
	cd contact-relationship-management-ui-with-svelte && make demo

.PHONY: demo-service
demo-service:
	cd contact-relationship-management-service-with-rust && make demo

.PHONY: demo-ui
demo-ui:
	cd contact-relationship-management-ui-with-svelte && make demo

.PHONY: test
test: test-github-discussions test-demo
	@echo "All tests passed!"

.PHONY: test-github-discussions
test-github-discussions:
	@echo "Running GitHub Discussions tests..."
	./bin/test-github-discussions

.PHONY: test-demo
test-demo:
	@echo "Running end-to-end demo tests..."
	./bin/test-demo
