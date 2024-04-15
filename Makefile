NPMJS_SCOPE=lucifayr
WASM_WEB_PKG_NAME = archwiki-web
WASM_WEB_PKG_DIR = wasm-web/pkg
WASM_NODEJS_PKG_NAME = archwiki-node
WASM_NODEJS_PKG_DIR = wasm-nodejs/pkg

CLI_VERSION=$(shell cargo metadata --no-deps --format-version 1 | jq -r '.packages[]  | select(.name == "archwiki-rs") | .version')
WASM_WEB_VERSION=$(shell jq -r '.version' $(WASM_WEB_PKG_NAME).json)
WASM_NODEJS_VERSION=$(shell jq -r '.version' $(WASM_NODEJS_PKG_NAME).json)

# check that version changed and required envs are set
check-version-cli : 
	! git diff --quiet Cargo.toml
	git diff Cargo.toml | grep  '+version = ' -B 2 | head -n 1 | grep -q ' name = "archwiki-rs"'

check-version-wasm-web :
	! git diff --quiet $(WASM_WEB_PKG_NAME).json
	git diff $(WASM_WEB_PKG_NAME).json | grep -q '+  "version": '

check-version-wasm-nodejs :
	! git diff --quiet $(WASM_NODEJS_PKG_NAME).json
	git diff $(WASM_NODEJS_PKG_NAME).json | grep -q '+  "version": '

build-wasm-web:
	wasm-pack build --release -t web -s $(NPMJS_SCOPE) --out-name $(WASM_WEB_PKG_NAME) --out-dir $(WASM_WEB_PKG_DIR)  ./ --features wasm-web --no-default-features
	jq -s '.[0] * .[1]'  $(WASM_WEB_PKG_DIR)/package.json $(WASM_WEB_PKG_NAME).json | sponge $(WASM_WEB_PKG_DIR)/package.json
	cp $(WASM_WEB_PKG_NAME)-README.md $(WASM_WEB_PKG_DIR)/README.md

build-wasm-nodejs:
	wasm-pack build --release -t nodejs -s $(NPMJS_SCOPE) --out-name $(WASM_NODEJS_PKG_NAME) --out-dir $(WASM_NODEJS_PKG_DIR) ./ --features wasm-nodejs --no-default-features
	jq -s '.[0] * .[1]'  $(WASM_NODEJS_PKG_DIR)/package.json $(WASM_NODEJS_PKG_NAME).json | sponge $(WASM_NODEJS_PKG_DIR)/package.json
	cp $(WASM_NODEJS_PKG_NAME)-README.md $(WASM_NODEJS_PKG_DIR)/README.md

build-wasm: build-wasm-web build-wasm-nodejs

bump-cli : check-version-cli build-wasm-web
	cargo build --release
	git add Cargo.toml Cargo.lock
	git commit -m "bump cli version to $(CLI_VERSION)"
	git tag "v$(CLI_VERSION)"

bump-wasm-web : check-version-wasm-web 
	wasm-pack pack $(WASM_WEB_PKG_DIR)
	git add $(WASM_WEB_PKG_NAME).json
	git commit -m "bump wasm-web version to $(WASM_WEB_VERSION)"
	git tag "wasm-web-v$(WASM_WEB_VERSION)"

bump-wasm-nodejs : check-version-wasm-nodejs build-wasm-nodejs
	wasm-pack pack $(WASM_NODEJS_PKG_DIR)
	git add $(WASM_NODEJS_PKG_NAME).json
	git commit -m "bump wasm-nodejs version to $(WASM_NODEJS_VERSION)"
	git tag "wasm-nodejs-v$(WASM_NODEJS_VERSION)"

bump-wasm : bump-wasm-web bump-wasm-nodejs

push-tags: 
	git push --follow-tags 
	git push github 
	git push --tags github

publish-cli : bump-cli push-tags
	cargo publish
	glab release create "v$(CLI_VERSION)" -n "cli v$(CLI_VERSION)"
	gh release create "v$(CLI_VERSION)" -t "cli v$(CLI_VERSION)" --generate-notes

publish-wasm-web : bump-wasm-web push-tags
	wasm-pack publish $(WASM_WEB_PKG_DIR)
	glab release create "wasm-web-v$(WASM_WEB_VERSION)" -n "wasm web v$(WASM_WEB_VERSION)"
	gh release create "wasm-web-v$(WASM_WEB_VERSION)" -t "wasm web v$(WASM_WEB_VERSION)" --generate-notes

publish-wasm-nodejs : bump-wasm-nodejs push-tags
	wasm-pack publish $(WASM_NODEJS_PKG_DIR)
	glab release create "wasm-nodejs-v$(WASM_NODEJS_VERSION)" -n "wasm nodejs v$(WASM_NODEJS_VERSION)"
	gh release create "wasm-nodejs-v$(WASM_NODEJS_VERSION)" -t "wasm nodejs v$(WASM_NODEJS_VERSION)" --generate-notes

publish-wasm : publish-wasm-web publish-wasm-nodejs