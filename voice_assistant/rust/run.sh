#!/usr/bin/env bash
set -e

IMAGE=${RUN_IMAGE:-voice-assistant-rust}
USER_UID=$(id -u)

if [[ "${RUN_IMAGE}" ]]; then
    docker pull "${IMAGE}"
fi

touch "${PWD}/.google_refresh_token"

CONTAINER=$(docker run -d \
    -e PULSE_SERVER=unix:/tmp/pulse.sock \
    -e SDL_AUDIODRIVER=pulse \
    -e SDL_VIDEODRIVER=dummy \
    --env-file .env \
    -v /run/user/${USER_UID}/pulse/native:/tmp/pulse.sock \
    -v ${HOME}/.claude:/root/.claude \
    -v ${HOME}/.claude.json:/root/.claude.json \
    -v ${PWD}/.orders_tokens:/app/.orders_tokens:rw \
    -v ${PWD}/.google_refresh_token:/app/.google_refresh_token:rw \
    ${IMAGE} "$@")

docker logs -f "$CONTAINER" &
wait $!
