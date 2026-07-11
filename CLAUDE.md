# Projects

## Default Instructions

Unless explicitly told otherwise, apply the following to all code written or modified.

### Workflow: Outside-In TDD with Gherkin

**Gherkin is required for ALL tests — unit, integration, and acceptance. No exceptions.**

Every feature, fix, or refactor follows this sequence. Do not skip or reorder steps:

1. **Specify** — Write or modify a **Gherkin `.feature` file** using Given/When/Then. Use the domain's ubiquitous language. This applies at every level:
   - **Unit level** — describe the behavior of a single aggregate, entity, value object, or domain service.
   - **Integration level** — describe the behavior of a repository, infrastructure adapter, or service composition.
   - **Acceptance level** — describe end-to-end user-facing behavior.
   - No code is written until the feature file is in place.

2. **Test** — Write the **step definitions and test harness** that execute the Gherkin scenario. The test must fail before implementation begins (red). No test is ever written without a corresponding `.feature` file.

3. **Implement** — Write the **minimum code** to turn the test green, following the DDD architecture and SOLID principles below. Refactor only after green.

### Test File Organization

Tests map one-to-one to the hexagonal architecture's top-level components:

- **Unit tests** — one `.feature` file and one step-definition file per **application service**. If a project has a single `VoiceListenerService`, it gets a single `voice_listener_service.feature` with a matching test harness. Unit-level scenarios for domain models (entities, value objects, domain services) that the application service delegates to live in the same feature file — don't create separate feature files for individual domain types.
- **Integration tests** — one `.feature` file and one step-definition file per **infrastructure adapter** (each dependency-injection binding). Every port implementation (e.g. the Piper TTS speaker, the Google STT transcriber, the cpal audio capturer) gets its own integration feature that exercises the real adapter against its external dependency.

This keeps the test suite's shape a direct mirror of the architecture: unit tests cover the service layer, integration tests cover the infrastructure layer.

### Architecture: Domain-Driven Design

Organize code around the domain model, not technical layers:

- **Ubiquitous Language** — Use the same terms in code as the business uses. Rename anything that drifts from the domain vocabulary.
- **Bounded Contexts** — Split large systems into contexts with clear boundaries. Each context owns its model; don't leak internals across boundaries.
- **Entities** — Objects defined by identity (an ID), not by attributes. Two entities with the same fields are not the same thing.
- **Value Objects** — Objects defined by their values, not identity. Immutable, equality by all fields. No IDs.
- **Aggregates** — Cluster entities and value objects behind a single root. The root enforces invariants; everything outside references the root only by ID.
- **Repositories** — Abstract persistence behind domain interfaces. Repositories return fully-hydrated aggregates, not database rows.
- **Domain Events** — Model significant business occurrences as explicit events. Use them to decouple side effects across contexts.
- **Application Services vs. Domain Services** — Application services orchestrate (no business logic). Domain services encapsulate logic that doesn't naturally belong to a single entity/value object.
- **Infrastructure Layer** — Keep frameworks, ORMs, HTTP clients, and external adapters out of the domain. Domain code imports nothing from infrastructure.

### Code-Level: SOLID Principles

- **S** — Single Responsibility: Each class, function, or module must have exactly one reason to change. Split anything that mixes concerns.
- **O** — Open/Closed: Design for extension without modifying existing code. Use polymorphism, strategy patterns, or configuration over hardcoding.
- **L** — Liskov Substitution: Subtypes must be fully substitutable for their base types. Don't weaken preconditions or strengthen postconditions in derived classes.
- **I** — Interface Segregation: Keep interfaces small and client-specific. No code should depend on methods it doesn't use.
- **D** — Dependency Inversion: Depend on abstractions (interfaces, protocols), not concrete implementations. Use dependency injection.

When suggesting or writing code, briefly note which principles are in play if it's not obvious.
