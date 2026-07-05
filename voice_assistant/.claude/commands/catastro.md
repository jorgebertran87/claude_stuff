---
description: Consulta datos catastrales de una propiedad por referencia catastral
allowed-tools: Bash
---

Usa el tool Bash para ejecutar SIEMPRE este comando y obtener los datos catastrales reales:

```bash
curl -s --max-time 15 -A "Mozilla/5.0" \
  "https://ovc.catastro.meh.es/OVCServWeb/OVCWcfCallejero/COVCCallejero.svc/json/Consulta_DNPRC?RefCat=REFERENCIA"
```

Sustituye `REFERENCIA` por la referencia catastral proporcionada ($ARGUMENTS) o extraída del contexto. La referencia catastral puede estar en dos formatos:
- **Urbana**: 20 caracteres (ej: `5989208UF6558N0003PX`)
- **Rústica**: 13 caracteres (ej: `13077A01800039`)

Si la respuesta JSON falla, usa el endpoint REST alternativo:
```bash
curl -s --max-time 15 -A "Mozilla/5.0" \
  "https://ovc.catastro.meh.es/OVCServWeb/OVCWcfCallejero/COVCCallejero.svc/rest/Consulta_DNPRC?RefCat=REFERENCIA"
```

## Interpretación de la respuesta JSON

Extrae y presenta estos campos:
- **Dirección**: `bico.bi.dt.ldt` — dirección postal completa
- **Uso**: `bico.bi.debi.luso` — tipo de uso (Residencial, Industrial, etc.)
- **Superficie**: `bico.bi.debi.sfc` — metros cuadrados construidos
- **Año construcción**: `bico.bi.debi.ant` — antigüedad
- **Coeficiente de participación**: `bico.bi.debi.cpt` — porcentaje sobre el total de la finca
- **Tipo de finca**: `bico.bi.finca.ltp` — descripción de la parcela
- **Construcciones**: array `bico.bi.lcons[]` — para cada una, mostrar:
  - `lcd`: localización (VIVIENDA, ALMACEN, etc.)
  - `dfcons.stl`: superficie en m²
  - `dvcons.dtip`: descripción del tipo constructivo
- **Mapa**: `bico.bi.finca.infgraf.igraf` — URL del visor cartográfico

## Formato de respuesta

Responde en texto plano, sin formato markdown. Presenta los datos agrupados:

Dirección: [dirección completa]
Uso: [tipo de uso]
Superficie construida: [sfc] m²
Año de construcción: [ant]
Coeficiente de participación: [cpt]%
Tipo de finca: [descripción]

Construcciones:
- [tipo]: [superficie] m² — [descripción]
...

Mapa: [URL del visor cartográfico]
