# services_controller

Control the services running on your server by **stable alias** — never touch
the underlying service names.

```
services_controller <start|stop|restart|status> <alias>
```

## Architecture (hexagonal / ports & adapters)

| Layer | File | Responsibility |
|---|---|---|
| Domain | `src/manager.rs` | `ServiceManager` — resolves an alias and delegates to the port |
| Domain | `src/registry.rs` | `ServiceRegistry` — alias → service map, loaded from YAML |
| Port | `src/control/mod.rs` | `ServiceController` trait — start/stop/restart/status |
| Adapter | `src/control/docker.rs` | `DockerController` — drives the Docker Engine HTTP API |
| Wiring | `src/main.rs` | builds the registry + adapter and runs a command |

**Docker is just one adapter.** The domain depends only on the
`ServiceController` port; swapping in systemd, a remote API, or anything else
is a new file under `src/control/` and one line of wiring — no domain change.

## Aliases

Declared in a YAML config file (see `aliases.example.yaml`):

```yaml
web: nginx
db: postgres
```

## Tests

Gherkin + cucumber, run in Docker:

```
make test              # unit
make test-integration  # integration
make test-all          # everything
```

- `features/service_manager.feature` — alias-addressed control (unit, fake controller)
- `features/alias_registry.feature` — loading aliases from YAML (integration, tempfile)
- `features/docker_control.feature` — the Docker adapter (integration, wiremock-mocked API)
