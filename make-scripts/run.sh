#!/usr/bin/env bash

set -euo pipefail

if ! type mprocs > /dev/null; then
    echo "Installing mprocs"
    cargo install mprocs
fi

if [[ -d ../outbox ]]; then
    mprocs "$MAKE run_server" "$MAKE run_outbox" "$MAKE watch"
else
    echo "$(tput bold)$(tput setaf 3)Warning: outboxd not started, you need to start it yourself if you want to send emails$(tput sgr0)"
    mprocs "$MAKE run_server" "$MAKE watch"
fi
