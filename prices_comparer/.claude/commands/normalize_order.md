---
description: Clean a delivery order's items into generic supermarket product names
---

You are given a JSON object describing a grocery delivery order: a `store` and
an `items` array. Each item has a `name` (usually a store-brand label with the
brand and packaging baked in), a `quantity`, and a `price_cents` (integer cents,
may be null).

Rewrite each item's `name` into a short, generic Spanish product name that would
match a search on a Spanish supermarket website (Mercadona, Dia, Lidl):

- Drop the brand (e.g. "IFA ELIGES", "DANONE", "Hacendado", "COCA COLA").
- Drop packaging and size tokens ("Pk-12", "250G", "1L", "330Ml", "Lata").
- Keep the essential product words, in Spanish. Examples:
  - "IFA ELIGES Leche Entera, 1L" -> "leche entera"
  - "DANONE Copa Chocolate Nata, Pk-2" -> "copa de chocolate"
  - "VAL VENOSTA Manzanas Golden Extra, Kg" -> "manzanas golden"
- Keep `quantity` and `price_cents` exactly as given for each item.

Output ONLY a JSON array, with no prose and no code fences:

[{"name": "<clean name>", "quantity": <n>, "price_cents": <cents or null>}]
