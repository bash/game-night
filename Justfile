cargo-flags := '--features development'

@default:
	just --list

up: build-web
	podman-compose up --build -d

down:
	podman-compose down

logs:
	podman-compose logs -f

build-web:
	podman build --tag game-night-public --target web_build --build-arg 'SASS_FLAGS=--embed-source-map --embed-sources' .
	@mkdir -p public/build
	podman run --rm --volume ./public/build:/srv game-night-public cp -rT /usr/local/src/game-night/public/ /srv/

fetch-live-db:
	scp root@fedora-01.infra.tau.garden:/opt/game-night/data/database.sqlite database.sqlite
	echo "DELETE FROM web_push_subscriptions;" | sqlite3 database.sqlite

publish:
	podman build --tag game-night --target publish .

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
