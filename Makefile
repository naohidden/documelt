.PHONY: up build test publish

up:
	docker compose up -d --build

build:
	docker exec documelt npm run build

test:
	docker exec documelt npm run test:rust
	docker exec documelt npm test

publish:
	docker exec documelt npm run build
	npm publish --ignore-scripts

publish-patch:
	npm version patch --no-git-tag-version
	docker exec documelt npm run build
	npm publish --ignore-scripts

publish-minor:
	npm version minor --no-git-tag-version
	docker exec documelt npm run build
	npm publish --ignore-scripts
