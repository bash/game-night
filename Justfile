cargo-flags := ''
prod-host := 'root@fedora-01.infra.tau.garden'
prod-service-name := 'game-night-v2'

@default:
	just --list

up: build-web
	podman-compose up --build -d

down:
	podman-compose down

logs:
	podman-compose logs -f

build-web:
	#!/usr/bin/env bash
	set -euo pipefail
	image_name=game-night-public-dev
	podman build --tag $image_name --target public_dev --build-arg 'SASS_FLAGS=--embed-source-map --embed-sources' .
	mkdir -p public/build
	just _export_image $image_name public/build

fetch-prod-db:
	scp {{prod-host}}:/opt/game-night/data/database.sqlite data/database.sqlite
	echo "DELETE FROM web_push_subscriptions;" | sqlite3 data/database.sqlite

publish:
	podman build --tag game-night --target publish .
	podman build --tag game-night-public --target public .

deploy: publish
	podman image scp game-night {{prod-host}}::
	ssh {{prod-host}} -C 'export SYSTEMD_COLORS=true; systemctl restart {{prod-service-name}} && systemctl status {{prod-service-name}}'
	just deploy-public

deploy-public:
	#!/usr/bin/env bash
	set -euo pipefail
	public_dir=$(mktemp -d)
	just _export_image game-night-public "$public_dir"
	rsync --archive --verbose --human-readable --delete --no-owner --no-group "$public_dir/" {{prod-host}}:/usr/local/share/game-night/public
	rm -rf -- "$public_dir"

certs:
	@mkdir -p data/certs
	(cd data/certs && mkcert localhost 127.0.0.1 ::1)

recreate-db:
	rm -f data/database.sqlite
	sqlite3 data/database.sqlite < schema.sql

sqlx-prepare:
	DATABASE_URL=sqlite://./data/database.sqlite cargo sqlx prepare -- {{cargo-flags}}

check: sqlx-prepare
	cargo clippy {{cargo-flags}}

_export_image image output_dir:
	#!/usr/bin/env bash
	set -euo pipefail
	container_id=$(podman create '{{image}}')
	rm -f '{{image}}.tar'
	podman export "$container_id" -o '{{image}}.tar'
	podman rm "$container_id"
	tar -C '{{output_dir}}' -xf '{{image}}.tar'
	rm -f '{{image}}.tar'

check-flake:
	podman build -f Containerfile.nix -v $(pwd):/usr/local/src/game-night .
