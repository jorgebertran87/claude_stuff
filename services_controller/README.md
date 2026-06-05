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
| Domain | `src/registry.rs` | `ServiceRegistry` — alias → service-directory map, loaded from YAML |
| Domain | `src/command.rs` | `ServiceCommand::parse` — pure parsing of `/start <alias>` etc. |
| Port | `src/control/mod.rs` | `ServiceController` + `CommandRunner` traits |
| Port | `src/telegram/mod.rs` | `TelegramGateway` trait + `TelegramBot` orchestration |
| Adapter | `src/control/compose.rs` | `ComposeController` — runs `docker compose` per service |
| Adapter | `src/control/system.rs` | `SystemCommandRunner` — executes commands via `tokio::process` |
| Adapter | `src/telegram/http.rs` | `HttpTelegramGateway` — Telegram Bot API over HTTP |
| Wiring | `src/main.rs` | builds the registry + adapters and runs CLI or bot |

**Docker is just one adapter.** The domain depends only on the
`ServiceController` port; swapping in systemd, a remote API, or anything else
is a new file under `src/control/` and one line of wiring — no domain change.
The compose adapter itself runs commands through the `CommandRunner` port, so
its command-building and `ps` parsing are unit-tested without invoking Docker.

## Aliases

Each alias maps to the directory holding that service's `docker-compose.yml`
(see `aliases.example.yaml`). The controller runs `docker compose -f
<dir>/docker-compose.yml <action>`.

```yaml
web: /srv/web
db: /srv/db
```

## Tests

Gherkin + cucumber, run in Docker:

```
make test              # unit
make test-integration  # integration
make test-all          # everything
```

- `features/service_manager.feature` — alias-addressed control (unit, fake controller)
- `features/compose_control.feature` — the docker compose adapter (unit, fake command runner)
- `features/telegram_command.feature` — bot command handling (unit, fake gateway + controller)
- `features/alias_registry.feature` — loading aliases from YAML (integration, tempfile)
