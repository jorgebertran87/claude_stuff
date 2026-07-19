# Phase 4: Frontend-Backend Integration — Remaining Steps

## Completed (Steps 1–3)

| Step | Status | What was done |
|------|--------|--------------|
| 1: NPC serialization + CORS | ✅ | `GameSession::npcs()`, `GameSession::map()`, `SessionResponse` includes NPCs, `MapResponse::from(&GameMap)`, `actix-cors` added, `Cors::permissive()` in main.rs |
| 2: StubQuestionAsker | ✅ | `features/question_asker.feature`, integration test, `StubQuestionAsker` maps "Sphinx"→"Who rules Mount Olympus?"/"Zeus", "Minotaur"→"Who built the labyrinth?"/"Daedalus" |
| 3: BattleRepository + adapter | ✅ | `features/battle_repository.feature`, integration test, `BattleRepository` trait (save/find), `InMemoryBattleRepository` (Mutex<HashMap>) |

All 36 scenarios pass (`make test` exit 0).

## Step 4: Wire container + battle HTTP endpoints ✅

### Spec → Test → Implement

**Feature**: `features/battle_api.feature` (new)

Scenarios:
- Interact with NPC starts a battle → returns NPC + question
- Interact with no NPC → no battle
- Answer correctly → Victory
- Answer incorrectly → Defeat
- Answer non-existent battle → 404

**Test**: `tests/integration/battle_api.rs` (new) — actix-web test server, resolves services through container

**Implementation**:

1. **`container.rs`**: `AppState` gains `battle_service: Arc<BattleService>` and `battle_repo: Arc<dyn BattleRepository>`. `build_state()` wires StubQuestionAsker → BattleService + InMemoryBattleRepository. Handlers access via `state.battle_service` / `state.battle_repo` instead of `state.service` (needs `state.service` → `state.game_service` rename, or add fields alongside).

2. **`dto.rs`**: `InteractResponse` gains `battle: Option<BattleResponse>`; new `BattleResponse { question }`, `BattleAnswerRequest { answer }`, `BattleAnswerResponse { outcome }`

3. **`handlers.rs`**: `interact` handler auto-starts battle when NPC found (calls `battle_service.start_battle()`, stores via `battle_repo.save()`). Theme hardcoded to "Greek mythology". NPC name → `Player::new(npc_name)`. New `answer_battle` handler (finds battle, answers, returns outcome). New `get_battle` handler.

4. **`routes.rs`**: add `POST /api/sessions/{id}/battle/answer`, `GET /api/sessions/{id}/battle`

**Files**: `features/battle_api.feature` (new), `tests/integration/battle_api.rs` (new), `container.rs`, `dto.rs`, `handlers.rs`, `routes.rs`, `config/Cargo-test.toml`, `Dockerfile`

## Step 5: Frontend ApiClient + MapScene integration ✅

### Spec → Test → Implement

**Feature**: `features/api_integration.feature` (new)

Scenarios:
- Join game → player at (0,0), map 5×5, NPC visible
- Server-validated movement → move east succeeds, move south blocked by wall
- Interaction triggers battle (if NPC adjacent)

**Test**: `tests/steps/api_integration.steps.ts` (new) — Playwright + Cucumber.js, requires backend + Vite running

**Implementation**:

1. **`src/services/ApiClient.ts`** (new): typed `fetch` wrapper — `join()`, `getSession()`, `move(direction)`, `interact()`, `answer(answer)`. Stores session ID internally. Base URL `http://localhost:8080`.

2. **`src/scenes/MapScene.ts`**: `create()` calls `apiClient.join()` → places NPCs from response data. `tryMove()` becomes async, calls `apiClient.move()`, animates on 200. Space key triggers `apiClient.interact()` → transitions to BattleScene if battle present.

3. **`src/game/Player.ts`**: `move()` renamed to `animateTo(targetX, targetY, direction, onComplete)` — removes `isWalkable` callback (collision is server-side).

4. **`src/main.ts`**: instantiate `ApiClient`, pass to scenes via game registry or scene data.

**Files**: `features/api_integration.feature` (new), `tests/steps/api_integration.steps.ts` (new), `src/services/ApiClient.ts` (new), `src/scenes/MapScene.ts`, `src/game/Player.ts`, `src/main.ts`, `Makefile`

## Step 6: BattleScene + DialogBox ✅

### Spec → Test → Implement

**Feature**: `features/battle_scene.feature` (new)

Scenarios:
- Question displayed in battle scene
- Correct answer → Victory shown
- Wrong answer → Defeat shown

**Test**: `tests/steps/battle_scene.steps.ts` (new) — Playwright drives the interact → battle → answer flow

**Implementation**:

1. **`src/scenes/BattleScene.ts`** (new): receives `{ npcName, question, sessionId }` via scene data. Renders dark overlay, NPC name, question text. Shows answer UI. Calls `apiClient.answer()`, displays outcome, transitions back to MapScene.

2. **`src/ui/BattleOverlay.ts`** (new): creates/removes DOM `<input>` + `<button>` positioned over the canvas. Returns Promise<string> with the player's answer.

3. **`src/main.ts`**: register `BattleScene` in scene list.

**Files**: `features/battle_scene.feature` (new), `tests/steps/battle_scene.steps.ts` (new), `src/scenes/BattleScene.ts` (new), `src/ui/BattleOverlay.ts` (new), `src/main.ts`

## Step 7: End-to-end acceptance test ✅

### Spec → Test → Implement

**Feature**: `features/full_game_loop.feature` (new)

Scenarios:
- Full loop: join → move east → interact → answer "Zeus" → Victory → return to map
- Defeat variant: answer wrong → Defeat → return to map

**Test**: `tests/steps/full_game_loop.steps.ts` (new) — Playwright drives the complete flow. Requires both backend (:8080) and Vite (:5173) running.

**Makefile update**: test target needs to start backend + Vite before running cucumber, kill both after.

**Files**: `features/full_game_loop.feature` (new), `tests/steps/full_game_loop.steps.ts` (new), `Makefile`

## Dependency Graph

```
Step 4 (Battle endpoints) ──> Step 5 (ApiClient + MapScene) ──> Step 6 (BattleScene) ──> Step 7 (E2E test)
```

## Key Backend Changes Still Needed (Step 4 details)

### container.rs refactoring
Current `AppState` has `service: Arc<GameWorldService>`. Need to either:
- (a) Rename `service` to `game_service` and add `battle_service` + `battle_repo` fields
- (b) Or keep `service` and add additional fields

Option (a) is cleaner but requires updating all handler references from `state.service` to `state.game_service`.

### Handler orchestration
The `interact` handler currently calls `game_service.interact()` which returns `Option<Npc>`. After Step 4, it should also:
1. If NPC found: build a `Player` from NPC name, call `battle_service.start_battle(&theme, &player)`, store via `battle_repo.save(session_id, battle)`
2. Return `InteractResponse { npc, battle }` with both fields

Theme is hardcoded: `Theme::new("Greek mythology").unwrap()`.

### DTO additions
```rust
#[derive(Debug, Serialize)]
pub struct BattleResponse {
    pub question: String,
}

#[derive(Debug, Deserialize)]
pub struct BattleAnswerRequest {
    pub answer: String,
}

#[derive(Debug, Serialize)]
pub struct BattleAnswerResponse {
    pub outcome: String,
}
```

`InteractResponse` changes from:
```rust
pub struct InteractResponse {
    pub npc: Option<NpcResponse>,
}
```
to:
```rust
pub struct InteractResponse {
    pub npc: Option<NpcResponse>,
    pub battle: Option<BattleResponse>,
}
```
