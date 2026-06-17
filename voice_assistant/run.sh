#!/usr/bin/env bash
set -e

IMAGE=${RUN_IMAGE:-voice-assistant-rust}
CONTAINER_NAME=voice-assistant
USER_UID=$(id -u)

if [[ "${RUN_IMAGE}" ]]; then
    docker pull "${IMAGE}"
fi

# Stop any already-running instance before starting a new one.
docker rm -f "$CONTAINER_NAME" 2>/dev/null || true

touch "${PWD}/.google_refresh_token"

CONTAINER=$(docker run -d \
    --name "$CONTAINER_NAME" \
    --restart unless-stopped \
    -e PULSE_SERVER=unix:/tmp/pulse.sock \
    -e SDL_AUDIODRIVER=pulse \
    -e SDL_VIDEODRIVER=dummy \
    --env-file .env \
    -v /run/user/${USER_UID}/pulse/native:/tmp/pulse.sock \
    -v ${PWD}/.claude:/app/.claude:ro \
    -v ${HOME}/.claude:/root/.claude \
    -v ${HOME}/.claude.json:/root/.claude.json \
    -v ${PWD}/.orders_tokens:/app/.orders_tokens:rw \
    -v ${PWD}/.google_refresh_token:/app/.google_refresh_token:rw \
    ${IMAGE} "$@")

# Forward signals to the container so Ctrl+C stops it cleanly.
cleanup() {
    echo "Stopping $CONTAINER_NAME..."
    docker stop "$CONTAINER_NAME" 2>/dev/null || true
}
trap cleanup INT TERM

docker logs -f "$CONTAINER" &
LOG_PID=$!
wait $LOG_PID
