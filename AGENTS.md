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

**Any top-level directory with a `Makefile` is a project** — discover projects from the
filesystem, never from a memorised list (tables like the one above go stale). Prefer the
project's own targets: `make test-all` when it exists, otherwise `make test`
(plus `make test-integration` where present).

**When multiple projects have changes, run all of their test suites in parallel** — issue all `make test` calls as parallel `Bash` tool calls in a single response, then wait for all results before proceeding.

**Never re-run a suite that is already green for the same code.** A green `make test-all`
(or `make test`) covers everything that follows until the code changes again — `/commit` and
`make deploy` must *not* trigger another full run on top of it. So:
- After a green `test-all`, when running `/commit` skip its test step and commit directly
  (the suite is already green); only run tests if code changed since.
- `make deploy` already gates on `make test`; do **not** run `test-all`/`test` yourself right
  before or after a deploy. Re-running a green suite wastes tokens and time.

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

## Deployment (production)

Production runs on a remote host (`pequenin`) from a **registry image** — the host pulls and runs it, it never builds. The three targets work together:

| Target | Runs where | What it does |
|---|---|---|
| `make build-prod` | dev | `docker buildx build --platform $(PROD_PLATFORM) -t $(DOCKER_USERNAME)/$(IMAGE) --push .` — build for the host's arch and push to the registry |
| `make deploy` | dev | `make test` → `make build-prod` → `scp` the run files to `pequenin:~/<project>/` |
| `make run-prod` | the host | `RUN_IMAGE=$(DOCKER_USERNAME)/$(IMAGE) docker compose up --no-build -d` — pull and run the pushed image (never builds), then follow logs |

- `deploy` always gates on green tests and a freshly pushed image before copying anything.
- Ship whatever `docker-compose.yml` needs at runtime: always `Makefile` + `docker-compose.yml`, plus per-project extras (`voice_assistant` ships `run.sh` and `.claude/`).
- `docker-compose.yml` must default the image to a local tag and honour a `RUN_IMAGE` override: `image: ${RUN_IMAGE:-<image>}`.
- Requires `DOCKER_USERNAME` and `PROD_PLATFORM` in `.env`, plus a registry login.
- Every image referenced at runtime (base images, sidecars like Selenium/Chrome) must support
  the arch in `PROD_PLATFORM` — currently `linux/amd64` (pequenin is an Intel i5 / 16 GB box).
  If the host ever changes arch again, flip `PROD_PLATFORM` in each project's `.env` and use
  the env overrides for single-arch sidecars (e.g. `CHROME_IMAGE=seleniarm/standalone-chromium`
  on ARM). Don't hardcode an architecture in docs or images — check `PROD_PLATFORM`.

## Verify Before Acting

Never act on an assumed fact about the repo or environment — check it first:

- Before referencing a file (`Cargo.lock`, a config, a DLL), confirm it exists (`ls`) and is
  tracked (`git ls-files`) — e.g. no `--locked` flags without a checked-in `Cargo.lock`.
- Before editing "the file that does X", read it and confirm it actually does X.
- Before relying on a tool, base image, or daemon behaviour, verify it on this machine
  (the sandbox, AppArmor, and snap Docker have surprised us before).
- If a signal pattern-matches a known failure, confirm the cause before applying the known fix.

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

## Skills

Use these slash commands instead of ad-hoc prompts — each one encodes the correct workflow for its task:

| Skill | When to use |
|---|---|
| `/tdd <feature>` | Building something new. Writes the Gherkin feature file first, stops for your approval, then implements and loops in Docker until all tests are green. Never writes production code before you confirm the spec. |
| `/refactor <goal>` | Changing existing code. Scopes the change upfront (stops for plan approval if >2 functions or >3 files), applies it, then self-corrects in a Docker loop until green. Never surfaces intermediate compile errors to you. |
| `/commit` | Committing. Detects all projects with changes, runs their test suites in parallel, and commits only if every project is green. |
| `/simplify` | Code-quality pass. Runs three parallel review agents (reuse, quality, efficiency) across the **whole codebase**, then applies the findings. |

### Skills encode workflow, not judgment

A skill is the correct *procedure*; it is not a substitute for thinking. While running one:

- If a hardcoded detail in the skill (project table, model name, path) conflicts with
  reality, **follow reality** — and update the skill file in the same session so it never
  bites again. Stale skills are bugs.
- If the request is broader or narrower than the skill assumes, adapt the procedure to the
  request, and say so.
- Prefer dynamic discovery (filesystem, Makefiles, git) over any list baked into a skill.

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
