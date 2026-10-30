# Repository tasks. Each target delegates to a script in bin/.

.PHONY: github-pages

# Publish contact-relationship-management.github.io/ to the github-pages remote (git subtree push).
github-pages:
	bin/make-github-pages
