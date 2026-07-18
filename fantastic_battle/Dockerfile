FROM rust:1-slim-bookworm AS test

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY config/Cargo-test.toml ./
RUN cat Cargo-test.toml >> Cargo.toml

COPY src/ src/
COPY tests/ tests/
COPY features/ features/

RUN touch src/lib.rs src/domain/mod.rs src/domain/model/mod.rs \
    src/domain/model/theme.rs src/domain/model/player.rs \
    src/domain/model/question.rs src/domain/model/battle.rs \
    src/domain/ports/mod.rs src/domain/ports/question_asker.rs \
    src/domain/service/mod.rs src/domain/service/battle_service.rs \
    tests/unit/battle_service.rs features/battle_service.feature \
    && cargo test --no-run --lib --test battle_service

CMD ["cargo", "test", "--lib", "--test", "battle_service"]
