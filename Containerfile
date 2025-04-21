FROM docker.io/rust:1.86 as builder
WORKDIR /usr/local/src/game-night
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y libsystemd-dev jq && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src/ ./src
COPY crates/ ./crates
COPY .sqlx/ ./.sqlx
ARG CARGO_BUILD_FLAGS=--release
RUN --mount=type=cache,target=/usr/local/src/game-night/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    build_messages=$(cargo build $CARGO_BUILD_FLAGS --color=always --message-format json) && \
    executable=$(echo "$build_messages" | jq --slurp --join-output '.[] | select(.reason == "compiler-artifact") | select(.target.name == "game-night") | .executable') && \
    cp "$executable" /usr/local/bin/

FROM registry.fedoraproject.org/fedora-minimal:42
COPY --from=builder /usr/local/bin/game-night /usr/local/bin/game-night

ENV ROCKET_SECRET_KEYS_PATH=/var/lib/game-night/keys
ENV ROCKET_DATABASES='{sqlite={url="/var/lib/game-night/database.sqlite"}}'
ENV ROCKET_TEMPLATE_DIR=/usr/local/share/game-night/templates
ENV ROCKET_EMAIL='{template_dir="/usr/local/share/game-night/emails",outbox_socket="/run/outbox/outbox.sock"}'
ENV ROCKET_WEB_PUSH='{template_dir="/usr/local/share/game-night/notifications"}'
ENV ROCKET_CONFIG=/usr/local/etc/game-night/Rocket.toml
WORKDIR /usr/local/share/game-night
COPY templates/ ./templates
COPY emails/ ./emails
RUN touch ./emails/email.css # TODO
COPY notifications/ ./notifications

WORKDIR /run/game-night
CMD ["game-night"]
