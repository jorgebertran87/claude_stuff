#!/usr/bin/env bash
set -e

IMAGE=${RUN_IMAGE:-voice-assistant-rust}
USER_UID=$(id -u)

CONTAINER=$(docker run -d \
    -e PULSE_SERVER=unix:/tmp/pulse.sock \
    -e SDL_AUDIODRIVER=pulse \
    -e SDL_VIDEODRIVER=dummy \
    --env-file .env \
    -v /run/user/${USER_UID}/pulse/native:/tmp/pulse.sock \
    -v ${HOME}/.claude:/root/.claude \
    -v ${HOME}/.claude.json:/root/.claude.json \
    -v ${PWD}/.orders_tokens:/app/.orders_tokens:rw \
    ${IMAGE} "$@")

docker logs -f "$CONTAINER" &
wait $!
