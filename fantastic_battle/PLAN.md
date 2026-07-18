# Plan: Browser-Based 2D Top-Down Exploration Game (Pokémon-Style)

## Progress

| Phase | Status | Commit |
|---|---|---|
| 0: Scaffolding | ✅ Done | `9ae70dc` |
| 1: GameWorld Domain Model | ⬜ Pending | — |
| 2: HTTP API | ⬜ Pending | — |
| 3: Frontend Foundation | ⬜ Pending | — |
| 4: Frontend-Backend Integration | ⬜ Pending | — |
| 5: UI Polish (Pokémon Effects) | ⬜ Pending | — |
| 6: Hardening & Deployment | ⬜ Pending | — |

To resume: read this file, check the git log for the last completed phase commit, and pick up from the next pending phase. The original plan snapshot is at `~/.claude/plans/twinkly-crunching-stroustrup.md`.

---

## Context

The current `fantastic_battle` is a text-based quiz-duel game (Rust library, DDD architecture, Gherkin-tested). The user wants it playable in the browser as a 2D top-down exploration game where the human player walks around a map, encounters AI NPCs, and battles them through quiz questions — all with Pokémon-style UI (dialog boxes, battle transitions, menu-based interaction).

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Game style | Top-down tile-based exploration (Pokémon-like) | User confirmed; matches "Pokémon UI effects" |
| Frontend tech | TypeScript + Phaser.js + Vite | Best browser game ecosystem; user confirmed |
| Backend tech | Rust + actix-web (keep existing domain) | Preserves existing DDD investment |
| State ownership | Client-authoritative movement, server-authoritative domain | Zero-latency movement; cheat-proof battles |
| Domain design | Separate bounded contexts (GameWorld + QuizBattle) | User confirmed; clean decoupling |
| Communication | REST for actions, WebSocket added later for push events | Start simple; REST covers MVP |
| Map format | Tiled editor → JSON export | Standard tooling for 2D tile maps |
| Project structure | Monorepo: `fantastic_battle/` (Rust backend) + `fantastic_battle_frontend/` (TS/Phaser) | Siblings under shared git repo |

## Architecture

```
fantastic_battle/          (Rust — existing, extended)
├── domain/
│   ├── model/             QuizBattle context: Player, Battle, Question, Theme (existing)
│   │   └── game_world/    GameWorld context: Position, Direction, Tile, TileMap,
│   │                       PlayerCharacter, Npc, GameSession (NEW)
│   ├── ports/             QuestionAsker (existing) + MapRepository, SessionRepository (NEW)
│   └── service/           BattleService (existing) + GameWorldService (NEW)
├── infrastructure/        actix-web HTTP, in-memory session store, static map loader (NEW)
└── container.rs           shaku DI wiring (NEW)

fantastic_battle_frontend/ (TypeScript + Phaser.js — NEW)
├── src/scenes/            BootScene, MapScene, BattleScene
├── src/game/              Player, Npc, MapManager, InputManager
├── src/services/          ApiClient
├── src/ui/                DialogBox, BattleUI
├── features/              Gherkin .feature files
└── tests/steps/           Cucumber.js + Playwright step definitions
```

## Implementation Phases

### Phase 0: Scaffolding ✅
- Add web server deps to `Cargo.toml` (actix-web, serde, uuid, shaku, tokio)
- Create `src/main.rs` binary entry point, `src/container.rs`, `src/infrastructure/`
- Initialize `fantastic_battle_frontend/` with Vite + TypeScript + Phaser 3 + Cucumber.js + Playwright
- Add minimal "hello world" Gherkin scenario on both sides to prove tooling

### Phase 1: GameWorld Domain Model (Backend)
- New value objects: `Position`, `Direction`, `TileType`, `Tile`
- New entities: `TileMap`, `PlayerCharacter`, `Npc`, `GameSession`
- New port traits: `MapRepository`, `SessionRepository`
- New application service: `GameWorldService` (join, move, interact)
- Gherkin `.feature` → step definitions → implementation for all domain behaviors
- Collision rules: walls block movement, map boundaries enforced

### Phase 2: HTTP API (Backend)
- actix-web routes: `POST /api/sessions`, `GET /api/sessions/{id}`, `POST .../move`, `POST .../interact`, `GET .../battle`, `POST .../battle/answer`
- In-memory `SessionRepository` adapter, static `MapRepository` adapter
- Integration tests: Gherkin scenarios driving HTTP requests against the test server, resolving services through `container::test_module()`

### Phase 3: Frontend Foundation
- Tile map rendering (Phaser tilemap from Tiled JSON export)
- Player sprite with 4-directional movement via arrow keys
- Client-side collision (check tilemap collision layer)
- NPC sprites placed on map
- Gherkin scenarios with Playwright asserting game state through `page.evaluate()`

### Phase 4: Frontend-Backend Integration
- `ApiClient` service calling backend endpoints
- MapScene wired to backend: join game, validate moves, trigger interaction
- BattleScene: Pokémon-style dialog box, question display, answer input, victory/defeat result
- End-to-end Gherkin acceptance test: full loop from join → explore → interact → battle

### Phase 5: UI Polish (Pokémon Effects)
- Dialog box with typewriter text effect and blinking advance indicator
- Battle transition: screen flash, NPC glow, scene switch
- Player walking animation (frame cycle), directional idle sprites
- Camera follows player, clamped to map bounds
- Sound: BGM, footsteps, battle jingles

### Phase 6: Hardening & Deployment
- Graceful error responses, server-side input validation
- Dockerfiles for both projects, docker-compose for orchestration
- Makefile targets: `make dev-backend`, `make dev-frontend`, `make test-all`

## TDD Workflow (Every Phase)

1. **Write `.feature` file** using domain language — no code before spec exists
2. **Write step definitions** — must fail (red)
3. **Implement minimum code** to go green
4. **Refactor** only after green
- Unit tests: one `.feature` per application service
- Integration tests: one `.feature` per port, resolve adapters through DI container
- Acceptance tests: end-to-end `.feature` in frontend project via Playwright

## Verification

- `make test` passes for backend (cargo test in Docker)
- `npm test` passes for frontend (Cucumber.js + Playwright)
- Manual: open browser, walk around map, encounter NPC, answer question, see victory/defeat
- End-to-end Playwright test exercises the full game loop

## MVP Scope

Phases 0–4 deliver a playable game: walk around a map, encounter AI NPCs, battle via quiz questions. Phases 5–6 add polish and production readiness.
