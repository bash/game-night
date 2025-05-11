FROM docker.io/rust:1.86 as builder
WORKDIR /usr/local/src/game-night
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y libsystemd-dev jq && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY askama.toml ./
COPY src/ ./src
COPY crates/ ./crates
COPY templates/ ./templates
COPY notifications/ ./notifications
COPY .sqlx/ ./.sqlx
ARG CARGO_BUILD_FLAGS=--release
RUN --mount=type=cache,target=/usr/local/src/game-night/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build $CARGO_BUILD_FLAGS --color=always && \
    cargo build $CARGO_BUILD_FLAGS --message-format json > _messages.json && \
    executable=$(jq --slurp --join-output '.[] | select(.reason == "compiler-artifact") | select(.target.name == "game-night") | .executable') < _messages.json && \
    cp "$executable" /usr/local/bin/


FROM docker.io/node:22 as npm_deps
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y brotli && rm -rf /var/lib/apt/lists/*
WORKDIR /usr/local/src/game-night
ENV NPM_CONFIG_UPDATE_NOTIFIER=false
ENV NPM_CONFIG_FUND=false
COPY package.json .
COPY package-lock.json .
RUN npm install


FROM npm_deps as email_css
COPY emails/email.scss ./emails/
RUN npx sass --no-source-map --style compressed emails/email.scss emails/email.css


FROM npm_deps as web_build
COPY browserslist .
COPY scss ./scss
COPY emails/email.scss ./emails/
COPY public ./public
COPY hash-files.py .
ARG SASS_FLAGS=--no-source-map
RUN npx sass scss/main.scss public/main.css $SASS_FLAGS
RUN npx lightningcss --browserslist public/main.css -o public/main.css
RUN npx sass --no-source-map --style compressed emails/email.scss emails/email.css
RUN cp node_modules/@github/relative-time-element/dist/bundle.js public/js/relative-time-element.js


FROM web_build as web_publish
RUN python3 hash-files.py
RUN gzip --keep --recursive public --best
RUN find public -type f -not -name '*.gz' -exec brotli --keep {} \+

FROM scratch as public_dev
COPY --from=web_build /usr/local/src/game-night/public/ .

FROM scratch as public
COPY --from=web_publish /usr/local/src/game-night/public/ .


FROM registry.fedoraproject.org/fedora-minimal:42 as runtime
COPY --from=builder /usr/local/bin/game-night /usr/local/bin/game-night

ENV ROCKET_DEFAULT_CONFIG=/usr/local/share/game-night/Rocket.toml
ENV ROCKET_CONFIG=/usr/local/etc/game-night/Rocket.toml
WORKDIR /usr/local/share/game-night
COPY config/Rocket.container.toml ./Rocket.toml
COPY emails/ ./emails
COPY --from=email_css /usr/local/src/game-night/emails/*.css ./emails/

WORKDIR /run/game-night
CMD ["game-night"]


FROM runtime as publish
# TODO: this is ugly, make path configurable
COPY --from=web_publish /usr/local/src/game-night/import-map.json /usr/local/bin/
COPY --from=web_publish /usr/local/src/game-night/asset-map.json /usr/local/bin/
