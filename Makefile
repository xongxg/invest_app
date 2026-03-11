PORT := 8082

dev:
	dx serve --platform web --port $(PORT)

build:
	dx build --platform web --release

.PHONY: dev build
