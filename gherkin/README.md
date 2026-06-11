# gherkin

A multi-language TDD kata: one shared Gherkin specification implemented independently in each language subdirectory, used to exercise the spec-first workflow (`/tdd`) outside the production projects.

## Layout

```
features/
└── domain/animal.feature   # the single source of truth for behaviour
golang/                     # Go implementation — table-driven `go test`
typescript/                 # TypeScript implementation — jest
```

`features/domain/animal.feature` defines the behaviour (hungry animals eating, species sounds) as scenario outlines. Each language directory contains its own domain code and a test suite that mirrors those scenarios, plus its own `Dockerfile`.

## Running the tests

Per the repo rule, nothing runs on the host — each implementation builds and tests inside Docker:

```bash
make -C golang test       # builds claude_tdd_go and runs `go test`
make -C typescript test   # builds claude_tdd_ts and runs jest
```

## Adding a language

1. Create `<language>/` with a `Dockerfile` whose default command runs the test suite.
2. Add a `Makefile` with `build` and `test` targets matching the existing ones.
3. Implement the scenarios from `features/domain/animal.feature` as native tests.
