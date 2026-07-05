---
description: Consulta datos catastrales de una propiedad por referencia catastral
---

Usa url_fetch para consultar la API del Catastro con esta URL exacta:

```
https://ovc.catastro.meh.es/OVCServWeb/OVCWcfCallejero/COVCCallejero.svc/json/Consulta_DNPRC?RefCat=REFERENCIA
```

Sustituye `REFERENCIA` por la referencia catastral proporcionada ($ARGUMENTS) o extraída del contexto. La referencia catastral puede estar en dos formatos:
- **Urbana**: 20 caracteres (ej: `5989208UF6558N0003PX`)
- **Rústica**: 13 caracteres (ej: `13077A01800039`)

Si la URL con `/json/` falla, prueba con `/rest/` (devuelve XML en lugar de JSON).

## Interpretación de la respuesta

El JSON devuelto contiene los datos catastrales no protegidos. Extrae y presenta estos campos:
- **Dirección**: `bico.bi.dt.ldt` — dirección postal completa
- **Uso**: `bico.bi.debi.luso` — tipo de uso (Residencial, Industrial, etc.)
- **Superficie construida**: `bico.bi.debi.sfc` — metros cuadrados
- **Año de construcción**: `bico.bi.debi.ant` — antigüedad
- **Coeficiente de participación**: `bico.bi.debi.cpt` — porcentaje sobre el total de la finca
- **Tipo de finca**: `bico.bi.finca.ltp` — descripción de la parcela
- **Superficie de suelo**: `bico.bi.finca.dff.ss` — metros cuadrados de parcela
- **Construcciones**: array `bico.bi.lcons[]` — para cada elemento, mostrar:
  - `lcd`: localización (VIVIENDA, ALMACEN, etc.)
  - `dfcons.stl`: superficie en m²
  - `dvcons.dtip`: descripción del tipo constructivo
- **Mapa**: `bico.bi.finca.infgraf.igraf` — URL del visor cartográfico

## Formato de respuesta

Responde en texto plano, sin formato markdown. Presenta los datos agrupados:

Dirección: [dirección completa]
Uso: [tipo de uso]
Superficie construida: [sfc] m²
Superficie de parcela: [ss] m²
Año de construcción: [ant]
Coeficiente de participación: [cpt]%
Tipo de finca: [descripción]

Construcciones:
- [tipo]: [superficie] m² — [descripción]
...

Mapa: [URL del visor cartográfico]
