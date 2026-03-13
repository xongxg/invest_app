PORT := 8082

dev:
	dx serve --platform web --port $(PORT) --package stock-frontend

build:
	dx build --platform web --release --package stock-frontend

.PHONY: dev build
