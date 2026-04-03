---
description: Consulta la próxima salida del autobús hacia Alameda Principal
allowed-tools: Bash
---

Consulta la API de la EMT Málaga con curl:

```bash
curl -s "https://navega.emtmalaga.es/api/estimaciones?codPar=1071&v=0.23"
```

Filtra los resultados por dirección "Alameda Principal". Para cada línea, muestra la próxima salida: en minutos si queda ≤30 min, o la hora exacta si es más tarde. Responde en texto plano, sin formato markdown.
