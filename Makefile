SCSS_FILES := $(shell find scss -name '*.scss')
MAIN_CSS := public/main.css
PRINT_CSS := public/print.css
SHELL := $(shell which bash)
SASS_FLAGS := --no-source-map

ifeq ($(env ENABLE_SOURCE_MAPS), true)
	SASS_FLAGS := --embed-source-map --embed-sources
endif


.ONESHELL:
.PHONY: all clean recreate-db

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

$(MAIN_CSS): $(SCSS_FILES)
	sass scss/main.scss $@ $(SASS_FLAGS)

$(PRINT_CSS): $(SCSS_FILES)
	sass scss/print.scss $@ $(SASS_FLAGS)
