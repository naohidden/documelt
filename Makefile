.PHONY: up build test publish demo make-gif

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

demo:
	docker exec -d documelt npx http-server /workspace -p 8080 --cors
	@echo "http://localhost:8080/docs/"

publish-minor:
	npm version minor --no-git-tag-version
	docker exec documelt npm run build
	npm publish --ignore-scripts

make-gif:
	ffmpeg -i sample.mp4 -vf "fps=10,scale=800:-1" docs/assets/demo.gif
