# Flights Scanner

A flights scanner HTTP server built in Rust following Domain-Driven Design (DDD) principles and Test-Driven Development (TDD).

All commands run inside Docker — no local Rust installation required.

---

## Architecture

```
src/
├── domain/          # Entities, value objects, aggregates, port traits (pure Rust, no I/O)
├── application/     # Use cases that orchestrate domain logic
└── infrastructure/  # Axum HTTP server, DTOs, adapters (Skyscanner + in-memory fallback)
```

The domain and application layers have zero knowledge of HTTP or any external system. They talk to the outside world exclusively through the `FlightSearchPort` trait. The active adapter is selected at startup based on environment variables — no code changes required to switch providers.

---

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)

---

## Run the tests

```bash
docker build --target tester -t flights-scanner-test .
```

This runs all 58 tests (domain unit tests + application integration tests + HTTP E2E tests + Skyscanner adapter tests) and enforces `cargo clippy -D warnings`. The build fails if any test fails or any warning is present.

---

## Run the server

```bash
# Build the runtime image
docker build -t flights-scanner .
```

**With the Skyscanner provider** (real flight data):

```bash
docker run -p 3000:3000 -e SKYSCANNER_API_KEY=your_key_here flights-scanner
```

**Without an API key** (falls back to in-memory preset data):

```bash
docker run -p 3000:3000 flights-scanner
```

The server binds to `0.0.0.0:3000` and logs which provider is active:

```
Using Skyscanner provider
Flights scanner listening on port 3000
```

or

```
SKYSCANNER_API_KEY not set — using in-memory provider
Flights scanner listening on port 3000
```

---

## API

### Health check

```
GET /health
```

```bash
curl http://localhost:3000/health
# → ok
```

---

### Search flights

```
POST /api/v1/flights/search
Content-Type: application/json
```

**Request body**

| Field          | Type     | Required | Description                                  |
|----------------|----------|----------|----------------------------------------------|
| `origin`       | string   | yes      | 3-letter IATA code (e.g. `"MAD"`)            |
| `destination`  | string   | yes      | 3-letter IATA code (e.g. `"LHR"`)            |
| `departure_date` | ISO 8601 | yes    | Must be in the future                        |
| `return_date`  | ISO 8601 | no       | Omit for one-way                             |
| `adults`       | integer  | yes      | ≥ 1                                          |
| `children`     | integer  | no       | Default 0                                    |
| `infants`      | integer  | no       | Default 0, cannot exceed `adults`            |
| `cabin_class`  | string   | yes      | `"Economy"`, `"Business"`, or `"First"`      |
| `max_price`    | float    | no       | Filter: maximum price in EUR                 |
| `max_stops`    | integer  | no       | Filter: maximum number of stops              |
| `sort_by`      | string   | no       | `"Price"` (default) or `"Duration"`          |

**Example — one-way search**

```bash
curl -s -X POST http://localhost:3000/api/v1/flights/search \
  -H "Content-Type: application/json" \
  -d '{
    "origin": "MAD",
    "destination": "LHR",
    "departure_date": "2026-12-01T10:00:00Z",
    "adults": 1,
    "cabin_class": "Economy"
  }' | jq
```

**Example — with filters**

```bash
curl -s -X POST http://localhost:3000/api/v1/flights/search \
  -H "Content-Type: application/json" \
  -d '{
    "origin": "MAD",
    "destination": "LHR",
    "departure_date": "2026-12-01T10:00:00Z",
    "adults": 2,
    "children": 1,
    "cabin_class": "Economy",
    "max_price": 150.0,
    "max_stops": 0,
    "sort_by": "Price"
  }' | jq
```

**Response**

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "outbound": {
      "number": "VY7654",
      "origin": "MAD",
      "destination": "LHR",
      "departure": "2026-12-01T14:00:00Z",
      "arrival": "2026-12-01T16:10:00Z",
      "cabin_class": "Economy"
    },
    "inbound": null,
    "price": { "amount": 99.99, "currency": "EUR" },
    "seats_available": 3,
    "is_round_trip": false,
    "total_duration_minutes": 130
  }
]
```

**Error responses**

| Status | When |
|--------|------|
| `400 Bad Request` | Invalid IATA code, same origin/destination, past departure date, invalid passenger count |
| `404 Not Found` | No flights match the criteria |
| `503 Service Unavailable` | Search provider is unavailable |

```json
{ "error": "invalid IATA code: 'INVALID'" }
```

---

## Preset data (in-memory adapter)

The server ships with five hardcoded routes for local development:

| Flight | Route     | Departure          | Duration | Price    |
|--------|-----------|--------------------|----------|----------|
| IB3456 | MAD → LHR | 2026-12-01 10:00   | 150 min  | €189.99  |
| VY7654 | MAD → LHR | 2026-12-01 14:00   | 130 min  | €99.99   |
| IB3457 | LHR → MAD | 2026-12-08 11:00   | 155 min  | €179.99  |
| VY1234 | BCN → CDG | 2026-12-02 07:00   | 90 min   | €79.99   |
| IB6250 | MAD → JFK | 2026-12-05 13:00   | 540 min  | €1850.00 (Business) |

Results are filtered by `origin` and `destination` before any use-case filters are applied.

---

## Adding another provider

1. Create a new adapter in `src/infrastructure/adapters/` that implements `FlightSearchPort`:

```rust
pub struct AmadeusAdapter { /* api key, http client, etc. */ }

#[async_trait]
impl FlightSearchPort for AmadeusAdapter {
    async fn search(&self, criteria: &SearchCriteria) -> Result<Vec<FlightOffer>, DomainError> {
        // call Amadeus API, map response to domain types
    }
}
```

2. Wire it in `src/main.rs` alongside the existing providers. No domain or application layer changes are needed.

---

## Test coverage summary

| Layer | Tests | What is covered |
|-------|-------|-----------------|
| Domain | 28 | Value object invariants, aggregate behaviour |
| Application | 9 | Use case filters, sorting, error propagation |
| HTTP (E2E) | 9 | Status codes, response shape, error mapping |
| Skyscanner mapper | 6 | Price conversion, one-way, round-trip, invalid data skipped |
| Skyscanner adapter | 6 | Polling flow, HTTP errors, failed/empty responses |
| **Total** | **58** | |
