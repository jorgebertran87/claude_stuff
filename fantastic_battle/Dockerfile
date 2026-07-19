FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src/ src/
RUN cargo build --release --bin fantastic_battle_server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/fantastic_battle_server /usr/local/bin/
EXPOSE 8080
ENV PORT=8080
CMD ["fantastic_battle_server"]


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
    src/domain/ports/battle_repository.rs src/domain/ports/npc_name_generator.rs \
    src/domain/service/mod.rs src/domain/service/battle_service.rs \
    src/domain/service/game_world_service.rs \
    src/infrastructure/mod.rs src/infrastructure/http/mod.rs \
    src/infrastructure/persistence/mod.rs src/infrastructure/map/mod.rs \
    src/infrastructure/question/mod.rs src/infrastructure/npc/mod.rs \
    tests/unit/battle_service.rs tests/unit/game_world_service.rs \
    src/infrastructure/map/static_map_repository.rs \
    src/infrastructure/persistence/in_memory_session_repository.rs \
    src/infrastructure/persistence/in_memory_battle_repository.rs \
    src/infrastructure/question/stub_question_asker.rs \
    src/infrastructure/question/deepseek_question_asker.rs \
    src/infrastructure/npc/stub_npc_name_generator.rs \
    src/infrastructure/npc/deepseek_npc_name_generator.rs \
    src/infrastructure/http/dto.rs \
    src/infrastructure/http/handlers.rs \
    src/infrastructure/http/routes.rs \
    tests/integration/http_server.rs tests/integration/game_world_api.rs \
    tests/integration/question_asker.rs tests/integration/battle_repository.rs \
    tests/integration/battle_api.rs tests/integration/error_handling.rs \
    tests/integration/npc_name_generator.rs \
    features/battle_service.feature features/server_health.feature \
    features/game_world_service.feature features/game_world_api.feature \
    features/question_asker.feature features/battle_repository.feature \
    features/battle_api.feature features/error_handling.feature \
    features/npc_name_generator.feature \
    && cargo test --no-run --lib --test battle_service --test game_world_service --test http_server --test game_world_api --test question_asker --test battle_repository --test battle_api --test error_handling --test npc_name_generator

CMD ["cargo", "test", "--lib", "--test", "battle_service", "--test", "game_world_service", "--test", "http_server", "--test", "game_world_api", "--test", "question_asker", "--test", "battle_repository", "--test", "battle_api", "--test", "error_handling", "--test", "npc_name_generator"]
