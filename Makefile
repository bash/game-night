SCSS_FILES := $(shell find scss -name '*.scss')
MAIN_CSS := public/main.css
PRINT_CSS := public/print.css
SHELL := $(shell which bash)
SASS_FLAGS := --no-source-map
PUBLISH_DIR := publish

ifeq ($(env ENABLE_SOURCE_MAPS), true)
	SASS_FLAGS := --embed-source-map --embed-sources
endif


.ONESHELL:
.PHONY: all clean recreate-db certs run publish

all: $(MAIN_CSS) $(PRINT_CSS)

clean:
	rm -f $(MAIN_CSS) $(PRINT_CSS)

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
	openssl req -x509 -newkey rsa:4096 -sha256 -days 3650 -nodes \
		-keyout private/key.pem \
		-out private/cert.pem \
		-subj "/CN=localhost" \
		-addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

run:
	ROCKET_TLS={certs="private/cert.pem",key="private/key.pem"} cargo run --features tls

$(MAIN_CSS): $(SCSS_FILES)
	sass scss/main.scss $@ $(SASS_FLAGS)

$(PRINT_CSS): $(SCSS_FILES)
	sass scss/print.scss $@ $(SASS_FLAGS)

publish:
	@rm -rf $(PUBLISH_DIR)
	@mkdir -p $(PUBLISH_DIR)
	cargo build --release
	cp target/release/game-night $(PUBLISH_DIR)/
	cp -R {public,templates,emails} $(PUBLISH_DIR)/
