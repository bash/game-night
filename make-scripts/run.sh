#!/usr/bin/env bash

set -euo pipefail

export CARGO_TERM_COLOR=always

if [[ -d ../outbox ]]; then
    parallel --lb --halt now,done=1 --tagstring [{}] ::: "$MAKE run_server" "$MAKE run_outbox" "$MAKE watch"
else
    echo "$(tput bold)$(tput setaf 3)Warning: outboxd not started, you need to start it yourself if you want to send emails$(tput sgr0)"
    "$MAKE" run_server
fi
