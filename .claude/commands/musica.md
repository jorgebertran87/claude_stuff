---
description: Busca una canción o playlist en Spotify y devuelve el comando para Alexa
allowed-tools: Bash, WebSearch
---

Busca en Spotify la canción o playlist: $ARGUMENTS

Reglas:
- Busca el nombre exacto proporcionado
- Para playlists, busca solo en inglés
- Verifica disponibilidad
- Devuelve únicamente: Alexa, pon "X" en Spotify
