SCSS_FILES := $(shell find scss -name '*.scss')
MAIN_CSS := public/main.css
EMAIL_CSS := emails/email.css
RELATIVE_TIME_ELEMENT := public/js/relative-time-element.js
SHELL := $(shell which bash)
SASS_FLAGS := --no-source-map
PUBLISH_DIR := publish
SASS := npx sass
LIGHTNING := npx lightningcss --browserslist
NPM_SENTINEL := node_modules/.sentinel
CARGO_FLAGS := --features development

ifeq ($(env ENABLE_SOURCE_MAPS), true)
	SASS_FLAGS := --embed-source-map --embed-sources
endif

.PHONY: all clean recreate-db certs run publish deploy redeploy check sqlx-prepare
.NOTPARALLEL: deploy

all: $(MAIN_CSS) $(EMAIL_CSS) $(RELATIVE_TIME_ELEMENT)

$(NPM_SENTINEL): package.json package-lock.json
	npm install
	@touch $(NPM_SENTINEL)

$(RELATIVE_TIME_ELEMENT): $(NPM_SENTINEL) node_modules/@github/relative-time-element/dist/bundle.js
	cp node_modules/@github/relative-time-element/dist/bundle.js $@

sqlx-prepare:
	DATABASE_URL=sqlite://./database.sqlite cargo sqlx prepare

check: sqlx-prepare
	cargo clippy $(CARGO_FLAGS)

clean:
	rm -rf $(PUBLISH_DIR)
	rm -f $(MAIN_CSS)
	rm -rf outbox/
	rm -rf node_modules/

watch:
	MAKE=$(MAKE) ./make-scripts/watch.sh

recreate-db:
	rm -f database.sqlite
	sqlite3 database.sqlite < schema.sql

certs:
	@mkdir -p private
	(cd private && mkcert localhost 127.0.0.1 ::1)

run:
	MAKE=$(MAKE) ./make-scripts/run.sh

run_outbox:
	@$(MAKE) -C ../outbox run

run_server:
	cargo run $(CARGO_FLAGS)

$(MAIN_CSS): $(SCSS_FILES) $(NPM_SENTINEL) browserslist
	$(SASS) scss/main.scss $@ $(SASS_FLAGS)
	$(LIGHTNING) $(MAIN_CSS) -o $(MAIN_CSS)

$(EMAIL_CSS): emails/email.scss $(NPM_SENTINEL)
	$(SASS) --no-source-map --style compressed $< $@

publish: all
	@rm -rf $(PUBLISH_DIR)
	@mkdir -p $(PUBLISH_DIR)
	podman build -t game-night-build .
	podman volume create --ignore game-night-cargo-registry
	podman run -t --rm -v game-night-cargo-registry:/root/.cargo/registry -v ./:/build:z --workdir /build game-night-build cargo build --release
	cp target/release/game-night $(PUBLISH_DIR)/
	cp -R {public,templates,emails} $(PUBLISH_DIR)/
	python3 hash-files.py
	find $(PUBLISH_DIR) -name '.DS_Store' -exec rm {} +
	gzip --keep --recursive $(PUBLISH_DIR)/public --best
	find $(PUBLISH_DIR)/public -type f -exec brotli --keep {} \+

deploy: publish redeploy

redeploy:
	rsync --archive --verbose --human-readable --delete --no-owner --no-group $(PUBLISH_DIR)/ root@fedora-01.infra.tau.garden:/opt/game-night/bin/
	ssh root@fedora-01.infra.tau.garden -C 'export SYSTEMD_COLORS=true; systemctl restart game-night && systemctl status game-night'
