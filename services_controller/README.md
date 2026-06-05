# services_controller

Control the services running on your server by **stable alias** — never touch
the underlying service names.

```
services_controller <start|stop|restart|status> <alias>   # one-shot CLI
services_controller bot                                    # Telegram bot
```

## Telegram bot

Run `services_controller bot` with `TELEGRAM_BOT_TOKEN` set to control services
from chat with per-action commands:

```
/start <alias>     /stop <alias>     /restart <alias>     /status <alias>
```

Set `TELEGRAM_ALLOWED_CHATS` (comma-separated chat IDs) to restrict who can
issue commands; leave it blank to allow any chat.

## Architecture (hexagonal / ports & adapters)

| Layer | File | Responsibility |
|---|---|---|
| Domain | `src/manager.rs` | `ServiceManager` — resolves an alias and delegates to the port |
| Domain | `src/registry.rs` | `ServiceRegistry` — alias → service map, loaded from YAML |
| Domain | `src/command.rs` | `ServiceCommand::parse` — pure parsing of `/start <alias>` etc. |
| Port | `src/control/mod.rs` | `ServiceController` trait — start/stop/restart/status |
| Port | `src/telegram/mod.rs` | `TelegramGateway` trait + `TelegramBot` orchestration |
| Adapter | `src/control/docker.rs` | `DockerController` — drives the Docker Engine HTTP API |
| Adapter | `src/telegram/http.rs` | `HttpTelegramGateway` — Telegram Bot API over HTTP |
| Wiring | `src/main.rs` | builds the registry + adapters and runs CLI or bot |

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
- `features/telegram_command.feature` — bot command handling (unit, fake gateway + controller)
- `features/alias_registry.feature` — loading aliases from YAML (integration, tempfile)
- `features/docker_control.feature` — the Docker adapter (integration, wiremock-mocked API)
