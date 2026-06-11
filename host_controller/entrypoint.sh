#!/bin/sh
# Runs as root only long enough to make the mounted SSH credentials (whatever
# uid owns them on the deploying host) readable by the unprivileged `bot` user,
# then hands the process over to it.
set -eu

key="${SSH_KEY:-/secrets/id_ed25519}"
known_hosts="${SSH_KNOWN_HOSTS:-/secrets/known_hosts}"

install -d -m 700 -o bot -g bot /run/ssh
install -m 400 -o bot -g bot "$key" /run/ssh/id_ed25519
install -m 644 -o bot -g bot "$known_hosts" /run/ssh/known_hosts

export SSH_KEY=/run/ssh/id_ed25519
export SSH_KNOWN_HOSTS=/run/ssh/known_hosts
export HOME=/home/bot

exec setpriv --reuid bot --regid bot --clear-groups /usr/local/bin/host_controller "$@"
