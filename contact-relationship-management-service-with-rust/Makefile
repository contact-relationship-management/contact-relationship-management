# Makefile for contact-relationship-management-service-with-rust
# Provides a convenient 'demo' target to start the service with migrations and seed data

.PHONY: demo
demo:
	@echo "Running database migrations..."
	cargo run -- db migrate
	@echo "Seeding database..."
	cargo run -- task seed
	@echo "Starting service..."
	cargo run -- start
