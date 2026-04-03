---
description: Consulta la predicción meteorológica de Málaga (AEMET)
allowed-tools: Bash
---

Consulta la predicción horaria de Málaga en AEMET con curl:

```bash
curl -s "https://www.aemet.es/es/eltiempo/prediccion/municipios/horas/tabla/malaga-id29067"
```

Resume el tiempo actual y las próximas horas: temperatura, lluvia, viento y cualquier aviso relevante. Responde en texto plano, sin formato markdown.
