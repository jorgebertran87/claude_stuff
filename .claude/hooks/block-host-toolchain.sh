#!/bin/sh
# PreToolUse hook (Bash): enforce the CLAUDE.md rule "Never run cargo, python,
# or any project toolchain directly on the host". Blocks a command when any of
# its segments starts with a toolchain binary; running the same binary inside
# Docker (docker run ... cargo test) or via make is unaffected, because there
# the segment starts with `docker`/`make`.
cmd=$(jq -r '.tool_input.command // empty')
if printf '%s\n' "$cmd" | grep -qE '(^|&&|\|\||;|\|)[[:space:]]*(sudo[[:space:]]+)?(cargo|rustc|rustup|pytest|pip3?|python3?)([[:space:]]|$)'; then
  echo "Blocked by .claude/hooks/block-host-toolchain.sh: never run the project toolchain on the host. Use the project's Makefile (make test / make test-all) or docker build + docker run instead — see CLAUDE.md." >&2
  exit 2
fi
exit 0
