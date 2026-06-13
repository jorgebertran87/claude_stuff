"""mitmproxy addon: capture the Glovo bearer token from live traffic.

Runs inside the `glovo_capture` docker-compose service. Every request your
phone makes to Glovo's API carries an `Authorization: Bearer <token>`
header; this addon writes that token to the file the prices_comparer bot
reads (`/data/glovo_token`, shared via the `glovo_state` volume), so
`/glovo` always uses a fresh token without anyone pasting anything.

The token is written atomically (temp file + rename) so the bot never
reads a half-written value, and only when it actually changes.
"""

import os

# Must match GLOVO_TOKEN_FILE in the bot (default /data/glovo_token).
TOKEN_FILE = os.environ.get("GLOVO_TOKEN_FILE", "/data/glovo_token")
GLOVO_HOST_SUFFIX = "glovoapp.com"

# Remember the last token written so we touch the disk only on change.
_last_written = None


def _store(token: str) -> None:
    global _last_written
    if token == _last_written:
        return
    os.makedirs(os.path.dirname(TOKEN_FILE), exist_ok=True)
    tmp = TOKEN_FILE + ".tmp"
    with open(tmp, "w") as f:
        f.write(token)
    os.replace(tmp, TOKEN_FILE)
    _last_written = token


def request(flow) -> None:
    host = flow.request.pretty_host
    if not host.endswith(GLOVO_HOST_SUFFIX):
        return
    auth = flow.request.headers.get("authorization")
    if auth and auth.strip():
        _store(auth.strip())
