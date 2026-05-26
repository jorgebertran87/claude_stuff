# Claude Code Agent — claude_tdd

## Build & Test Environment

**Never run `cargo`, `python`, or any project toolchain directly on the host.**
All builds and tests run inside Docker. Per-project commands:

| Project | Build | Test |
|---|---|---|
| `changes_detector` | `docker build --target test -t changes-detector-test .` | `docker run --rm changes-detector-test cargo test` |
| `flights_scanner` | `docker build -t flights-scanner-test --target test .` | `docker run --rm flights-scanner-test cargo test` |
| `voice_assistant` | `docker build --platform linux/amd64 --target test -t voice-assistant-test .` | `docker run --rm voice-assistant-test cargo test` |
| `minesweeper` | `docker build --target test -t minesweeper-test .` | `docker run --rm minesweeper-test python -m pytest` |

Run the project's `Makefile` targets when they exist (`make test`, `make test-integration`).

### Mutation Testing (Rust projects)

Use `cargo-mutants` via the `mutants` Docker stage to verify test suite quality:

| Target | Command | What it checks |
|---|---|---|
| Unit mutants | `make mutants` | Domain/application logic (`detector`, `monitor`, `runner`) |
| Integration mutants | `make mutants-integration` | Infrastructure layer (`source/`) |
| Single file | `make mutants-integration FILE=src/source/flare.rs` | One infrastructure file only |

Config files live in `config/mutants.toml` (unit) and `config/mutants-integration.toml` (integration).
- Unit config excludes `src/main.rs` and `src/source/**` (infrastructure not covered by unit tests).
- Integration config excludes `src/main.rs` and domain files (domain not covered by integration tests).

After any code change in a Rust file: build and run the affected test suite before declaring done or suggesting a commit. If tests fail, fix the errors yourself — do not surface compile errors for the user to report back.

## Change Scope Discipline

Make targeted, minimal fixes. Default to the smallest possible diff that solves the problem.

- If a bug is in one function, fix **that function only** — do not refactor adjacent code.
- If asked to do a simple transformation (e.g. "replace tabs with spaces"), do exactly that and nothing else.
- Before rewriting more than 2 functions or touching more than 3 files, **stop and present a plan** explaining why broader scope is needed.
- When running `/simplify` or a code-review pass, apply it to the **whole module or codebase**, not just recently changed files.

## Rust Workflow

- After editing a function signature or trait, search for **all call sites** (including integration tests) and update them in the same change.
- Architecture convention: hexagonal / ports-and-adapters. Business logic in `domain/`, infrastructure in `infrastructure/` or `src/source/`, wiring in `main.rs`.
- Test pattern: Gherkin feature files + cucumber for both unit (fake infrastructure) and integration (real I/O with mocks). See `voice_assistant` and `changes_detector` for reference.
- `serde_json = "1"` is always a dev-dependency when using `wiremock`.

## Commit Policy

**Never commit without green tests.** Run the full test suite in Docker first. If anything fails or is unexpectedly skipped, stop and show the output before committing.

Use the `/commit` skill — it enforces this automatically.

## Artifact Targeting

Edit the correct layer:
- If asked to change behaviour → edit **Rust source code** (`src/`)
- If asked to change a test → edit the **feature file or test `.rs`** in `tests/` or `features/`
- If asked to change a Claude skill → edit the file in `.claude/commands/`
- When uncertain, **ask which file to change** rather than guessing

## Multi-Agent Scope

When asked to run agents in parallel (e.g. `/simplify`, review tasks):
- Assign one agent per **top-level module or subdirectory**, not per changed file
- Each agent must verify its slice compiles and tests pass before reporting
- Aggregate all findings before applying any changes
