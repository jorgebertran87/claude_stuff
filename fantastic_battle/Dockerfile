FROM rust:1-slim-bookworm AS test

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY config/Cargo-test.toml ./
RUN cat Cargo-test.toml >> Cargo.toml

COPY src/ src/
COPY tests/ tests/
COPY features/ features/

RUN touch src/main.rs src/lib.rs src/container.rs \
    src/domain/mod.rs src/domain/model/mod.rs \
    src/domain/model/theme.rs src/domain/model/player.rs \
    src/domain/model/question.rs src/domain/model/battle.rs \
    src/domain/model/game_world/mod.rs \
    src/domain/model/game_world/position.rs \
    src/domain/model/game_world/direction.rs \
    src/domain/model/game_world/tile_type.rs \
    src/domain/model/game_world/game_map.rs \
    src/domain/model/game_world/npc.rs \
    src/domain/model/game_world/game_session.rs \
    src/domain/ports/mod.rs src/domain/ports/question_asker.rs \
    src/domain/ports/map_repository.rs src/domain/ports/session_repository.rs \
    src/domain/service/mod.rs src/domain/service/battle_service.rs \
    src/domain/service/game_world_service.rs \
    src/infrastructure/mod.rs src/infrastructure/http/mod.rs \
    src/infrastructure/persistence/mod.rs src/infrastructure/map/mod.rs \
    tests/unit/battle_service.rs tests/unit/game_world_service.rs \
    tests/integration/http_server.rs \
    features/battle_service.feature features/server_health.feature \
    features/game_world_service.feature \
    && cargo test --no-run --lib --test battle_service --test game_world_service --test http_server

CMD ["cargo", "test", "--lib", "--test", "battle_service", "--test", "game_world_service", "--test", "http_server"]
