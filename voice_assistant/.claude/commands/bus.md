---
description: Consulta la próxima salida del autobús hacia Alameda Principal
allowed-tools: Bash
---

Usa el tool Bash para ejecutar SIEMPRE este comando y obtener los horarios reales de la EMT Málaga:

```bash
curl -s "https://navega.emtmalaga.es/api/estimaciones?codPar=1071&v=0.23"
```

Si se proporcionó un código de parada ($ARGUMENTS), úsalo en la URL en lugar de 1071 y muestra todas las líneas y direcciones disponibles. Si no se proporcionó ninguno, filtra los resultados por dirección "Alameda Principal". Para cada línea, muestra la próxima salida: en minutos si queda ≤30 min, o la hora exacta si es más tarde. Responde en texto plano, sin formato markdown.
