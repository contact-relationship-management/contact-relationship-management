##
# Makefile for contact-relationship-management.
##

SERVICE := contact-relationship-management-service-with-rust
UI := contact-relationship-management-ui-with-svelte

##
# make: Build everything,
##
.PHONY: default
default:
	cd $(SERVICE) && make
	cd $(UI) && make demo

##
# make help: Display this help message.
##
.PHONY: help
help:
	@awk '/^##/{a=1-a}a' $(MAKEFILE_LIST) | cut -c3-

# newline: This is a print helper when you want to embed a newline in a message.
define newline


endef

##
# make check: Check this project, such as printing some of its important tools, their versions, and paths. This task is intended to help with installation, debugging, and maintenance.
##
.PHONY: check
check::
	@echo "Check this project, such as printing some of its important tools, their versions, and paths. This task is intended to help with installation, debugging, and maintenance."

check:: check-uname check-cargo check-pnpm

##
# make check-uname: Print the uname information of the system.
##
.PHONY: check-uname
check-uname:
	@echo "\n### uname ###"
	uname -a

##
# make check-cargo: Print the version and path of the command `cargo`.
##
.PHONY: check-cargo
check-cargo:
	@echo "\n### cargo ###"
	cargo --version || true
	which -a cargo || true

##
# make check-pnpm: Print the version and path of the command `pnpm`.
##
.PHONY: check-pnpm
check-pnpm:
	@echo "\n### pnpm ###"
	pnpm --version || true
	which -a pnpm || true

##
# make lint: Proofread everything.
##
.PHONY: lint
lint: lint-markdown

##
# make lint-markdown: Proofread Markdown files.
##
.PHONY: lint-markdown
lint-markdown:
	@bash scripts/lint-markdown.sh check --quiet

##
# make publish-github-pages: Publish the website subproject to its own sibling repository.
##
.PHONY: publish-github-pages
publish-github-pages:
	bin/publish-github-pages

##
# make run-demo: Start the demo.
##
.PHONY: run-demo
run-demo: run-demo-service run-demo-ui

##
# make run-demo-service: Start the demo back-end service.
##
.PHONY: run-demo-service
run-demo-service:
	cd $(SERVICE) && make demo

##
# make run-demo-ui: Start the demo front-end UI.
##
.PHONY: run-demo-ui
run-demo-ui:
	cd $(UI) && make demo

##
# make test: Test everything.
##
.PHONY: test
test: test-service test-ui test-github-discussions test-demo

##
# make test-service: Test the back-end service.
##
.PHONY: test-service
test-service:
	cd $(SERVICE) && make test

##
# make test-ui: Test the front-end UI.
##
.PHONY: test-ui
test-ui:
	cd $(UI) && make test

##
# make test-demo: Test the demo.
##
.PHONY: test-demo
test-demo:
	./bin/test-demo

##
# make test-github-discussions: Test GitHub Discussions.
##
.PHONY: test-github-discussions
test-github-discussions:
	./bin/test-github-discussions
