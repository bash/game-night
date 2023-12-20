SCSS_FILES := $(shell find scss -name '*.scss')
MAIN_CSS := public/main.css
PRINT_CSS := public/print.css
EMAIL_CSS := emails/email.css
SHELL := $(shell which bash)
SASS_FLAGS := --no-source-map
PUBLISH_DIR := publish

ifeq ($(env ENABLE_SOURCE_MAPS), true)
	SASS_FLAGS := --embed-source-map --embed-sources
endif

.ONESHELL:
.PHONY: all clean recreate-db certs run publish deploy

all: $(MAIN_CSS) $(PRINT_CSS) $(EMAIL_CSS)

clean:
	rm -f $(MAIN_CSS) $(PRINT_CSS)
	rm -rf outbox/

watch:
	@while true; do
		find scss -name '*.scss' | entr -d $(MAKE)
		@test $$? -ne 2 && break
	@done

recreate-db:
	rm -f database.sqlite
	sqlite3 database.sqlite < schema.sql

certs:
	@mkdir -p private
	(cd private && mkcert localhost 127.0.0.1 ::1)
run:
	ROCKET_TLS={certs="private/localhost+2.pem",key="private/localhost+2-key.pem"} cargo run --features development

run-sms-outbox:
	op run --env-file crates/sms-outbox/twilio.env \
		-- cargo run -p sms-outbox --color=always

$(MAIN_CSS): $(SCSS_FILES)
	sass scss/main.scss $@ $(SASS_FLAGS)

$(PRINT_CSS): $(SCSS_FILES)
	sass scss/print.scss $@ $(SASS_FLAGS)

$(EMAIL_CSS): emails/email.scss
	sass --no-source-map --style compressed $< $@

publish: all
	@set -e
	@rm -rf $(PUBLISH_DIR)
	@mkdir -p $(PUBLISH_DIR)
	podman build -t game-night-build .
	podman volume create --ignore game-night-cargo-registry
	podman run --rm -v game-night-cargo-registry:/root/.cargo/registry -v ./:/build:z --workdir /build game-night-build cargo build --release
	cp target/release/game-night $(PUBLISH_DIR)/
	cp -R {public,templates,emails} $(PUBLISH_DIR)/
	find $(PUBLISH_DIR) -name '.DS_Store' -exec rm {} +

deploy: publish
	rsync --archive --verbose --human-readable --delete $(PUBLISH_DIR)/ root@fedora-01.infra.tau.garden:/opt/game-night/bin/
