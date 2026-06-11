# minesweeper

A Python + OpenCV service that parses Minesweeper screenshots into a text board. Send it a phone screenshot (e.g. a photo shared through Telegram) and it returns the grid as unicode — every cell located, classified, and rendered.

```
■ ■ 2 2 ⚑ 1 1 1 1 2 2        ■  unrevealed
■ ■ 2 2 1 1 · · 1 2 ⚑        ⚑  flagged
1 2 ⚑ 1 · · · · 2 ⚑ 3        ·  revealed empty
· 1 1 1 · · · 1 3 ⚑ 2        1-8  revealed number
```

## How it works

```
screenshot ──► header_detector ──► grid_analyzer ──► cell_classifier ──► renderer
               skip the app UI     find the grid     per-cell state      unicode board
                                   bounds & cells
```

1. `header_detector` finds where the game header ends so the UI chrome is ignored.
2. `grid_analyzer` locates the board's bounding box and the cell lattice inside it.
3. `cell_classifier` labels each cell (unrevealed, flagged, empty, or digit) from its pixels.
4. `board_parser` assembles the cells into a `Board`; `renderer` prints it as text.

## Usage

Everything runs in Docker (`make build` produces the `minesweeper-detector` image).

**Parse one screenshot from the CLI:**

```bash
make parse SCREENSHOT=/path/to/screenshot.jpg
```

**Run the HTTP server** (the container's default command — gunicorn on port 5000):

| Endpoint | Method | Behaviour |
|---|---|---|
| `/parse` | POST (raw image bytes in the body) | `200` with the rendered board as plain text, `500` with the error message |
| `/health` | GET | `ok` |

```bash
docker run --rm -p 5000:5000 minesweeper-detector
curl --data-binary @screenshot.jpg http://localhost:5000/parse
```

## Tests

```bash
make test
```

The suite is pytest-bdd: one Gherkin feature per real screenshot in `tests/images/`, each asserting the **full rendered board** character-for-character. Adding a new tricky screenshot means dropping the image in `tests/images/`, writing its expected board in a feature file, and listing it in the `Makefile`'s `TEST_IMAGES`.

## Project structure

```
src/
├── main.py              # CLI entry point — parse one image and print the board
├── server.py            # Flask app — POST /parse, GET /health
├── detector.py          # Input adapters: image path / raw bytes → parsed Board
├── header_detector.py   # Locate the end of the app header
├── grid_analyzer.py     # Find grid bounds and cell geometry
├── cell_classifier.py   # Classify each cell's state from pixels
├── board_parser.py      # Cells → Board
├── renderer.py          # Board → unicode text
└── models.py            # CellState and board model types
```
