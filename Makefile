SCSS_FILES := $(shell find scss -name '*.scss')
MAIN_CSS := public/main.css
SHELL := $(shell which bash)

.ONESHELL:
.PHONY: all clean

all: $(MAIN_CSS)

clean:
	rm -f $(MAIN_CSS)

watch:
	@while true; do
		find scss -name '*.scss' | entr -d $(MAKE)
		@test $$? -ne 2 && break
	@done

$(MAIN_CSS): $(SCSS_FILES)
	sass scss/main.scss $@ --embed-source-map
