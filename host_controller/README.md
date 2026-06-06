# host_controller

Run **non-root** shell commands on the **parent host** from a Telegram chat.
The service is containerized; it reaches the host over **SSH** as a dedicated
unprivileged user. Docker isolates the bot logic; SSH is the controlled,
authenticated bridge to the host. "Non-root only" is enforced by the host: the
bot logs in as a normal user with **no sudo**.

```
You → Telegram → container polls getUpdates → authorize sender
                                            → ssh botuser@host "<command>"
                                            → reply (chunked) with stdout/stderr/exit
```

## ⚠️ This is a remote-command endpoint

Anyone who can message the bot and passes the allowlist gets a shell as
`botuser`. The allowlist is the only thing between the bot and the world:

- `TELEGRAM_ALLOWED_CHATS` **defaults to deny-all** when blank. Add only your
  own chat ID(s).
- Keep `TELEGRAM_BOT_TOKEN` and the SSH key secret (`.env` and `secrets/` are
  gitignored).
- `botuser` must have **no sudo** and key-auth only.

## Architecture (hexagonal / ports & adapters)

| Layer | File | Responsibility |
|---|---|---|
| Domain | `src/authorizer.rs` | allowlist check on the sender's chat ID |
| Domain | `src/request.rs` | parse a Telegram message into a command to run |
| Domain | `src/formatter.rs` | format a command result into Telegram-safe text (chunk/truncate) |
| Port | `src/executor/mod.rs` | `CommandExecutor` + `CommandRunner` traits |
| Port | `src/telegram/mod.rs` | `TelegramGateway` trait + bot poll loop |
| Adapter | `src/executor/ssh.rs` | builds `ssh …` and runs it via `CommandRunner` |
| Adapter | `src/executor/system.rs` | executes commands via `tokio::process` |
| Adapter | `src/telegram/http.rs` | Telegram Bot API over HTTP |
| Wiring | `src/main.rs` | builds adapters and runs the bot |

The SSH adapter shells out through the `CommandRunner` port, so its argument
building and output handling are unit-tested without invoking real `ssh`.

## Host setup (one time)

On the host, create the unprivileged bot user and authorize a key:

```sh
sudo useradd --create-home --shell /bin/bash botuser   # NOT in sudoers
sudo -u botuser ssh-keygen -t ed25519 -f /home/botuser/.ssh/id_ed25519 -N ''
sudo -u botuser sh -c 'cat /home/botuser/.ssh/id_ed25519.pub >> /home/botuser/.ssh/authorized_keys'
```

Copy the **private** key and the host's public key into `secrets/`:

```sh
mkdir -p secrets
sudo cp /home/botuser/.ssh/id_ed25519 secrets/id_ed25519
ssh-keyscan -p 22 <host> > secrets/known_hosts
```

Ensure `sshd` allows key auth and the bot user has no sudo.

## Configure & run

```sh
cp .env.example .env        # fill in TELEGRAM_BOT_TOKEN, TELEGRAM_ALLOWED_CHATS, SSH_*
make run                    # docker compose up --build -d && logs -f
```

## Tests

Gherkin + cucumber, run in Docker (never run cargo on the host):

```
make test              # unit
make test-integration  # integration
make test-all          # everything
make mutants           # mutation-test the domain
```
