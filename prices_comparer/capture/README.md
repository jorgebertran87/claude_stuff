# Automatic Glovo token capture

Glovo has no public API and no "log in with Glovo" for third parties, so the
bot can't fetch a token on its own. Instead it reads the token from a file
(`/data/glovo_token`, on the shared `glovo_state` volume) and the
`glovo_capture` mitmproxy service writes that file by watching your own
Glovo traffic.

```
phone (Glovo app)  ──HTTP proxy──▶  glovo_capture (mitmproxy on pequenin)
                                          │ writes the Bearer token
                                          ▼
                                   /data/glovo_token   ◀── reads it per /glovo
                                          ▲
                                   prices_comparer bot
```

Once set up, opening the Glovo app refreshes the token automatically; the
bot picks up the new value on the next `/glovo` with no restart.

## One-time setup

1. **Start the stack** (`make run`). mitmproxy listens on port `8080` and
   generates its CA under `/data/mitmproxy` on first run.

2. **Point your phone's HTTP proxy** at pequenin's Tailscale IP, port `8080`
   (Wi-Fi settings → proxy → manual). Both devices are already on your
   tailnet.

3. **Install the mitmproxy CA on the phone.** With the proxy active, open
   `http://mitm.it` in the phone browser and follow the steps for your OS.
   On Android you also need to mark it as trusted for apps (system trust
   store), which on modern Android means a rooted device — see the caveat.

4. **Open the Glovo app and load your orders.** The addon spots the
   `Authorization` header and writes the token. Send `/glovo` to the bot to
   confirm.

## The cert-pinning caveat (read before you start)

This is the make-or-break. Glovo's mobile app very likely **pins its TLS
certificate**, meaning it rejects the mitmproxy CA even once installed, and
no token is captured. Getting past that needs one of:

- a **rooted Android / jailbroken iOS** device plus a pinning-bypass
  (Frida / objection, or LSPosed + TrustMeAlready), or
- a **patched APK** with pinning stripped (apk-mitm / objection patchapk).

If your phone isn't rooted, automatic capture from the **app** won't work.
Two fallbacks that need no rooting:

- **Glovo web** (`glovoapp.com` in a desktop browser through the same
  proxy, or just read the `Authorization` header from the browser's
  network tab) — browsers honour a user-installed CA with no pinning fight.
- **Manual paste:** capture the token once by any means and send it to the
  bot with `/glovo_token <token>`. It lands in the same file and behaves
  exactly like an auto-captured one.

## When the token expires

Glovo tokens are short-lived. The bot detects a rejected token and replies
that it has expired. Just open the Glovo app again (capture refreshes it) or
paste a new one with `/glovo_token`.
