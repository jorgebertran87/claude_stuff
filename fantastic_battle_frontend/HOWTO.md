# Fantastic Battle — How to Play

Fantastic Battle is a Pokémon-style browser RPG where you explore a tile-based map, talk to NPCs, and fight battles by answering trivia questions.

## Quick Start

```bash
# Install dependencies
npm install

# Start the dev server
make dev
# or: npx vite
```

Open **http://localhost:5173** in your browser.

For the full experience (NPC interactions and battles), start the backend server first:

```bash
make start-backend
make dev
```

Without the backend, the game runs in offline mode with a single NPC named **Sphinx** to walk up to — but battles will not trigger.

---

## Controls

| Key | Action |
|---|---|
| **Arrow keys** (↑ ↓ ← →) | Move your character north, south, east, or west |
| **Space** | Interact with an adjacent NPC / advance dialog / dismiss dialog |
| **Enter** | Submit your answer during battle |
| Click **Answer** button | Submit your answer during battle |

---

## The Map

The game world is a **15 × 10 tile grid**. Your character starts at the top-left corner (0, 0), facing south.

- **Green tiles** — walkable ground.
- **Grey tiles** — walls and obstacles. You cannot walk through them.
- **Red squares** — NPCs standing on the map.

The camera follows your character as you move. You cannot step off the edge of the map.

---

## How to Play

### 1. Explore the map

Use the arrow keys to walk around. Each movement advances your character one tile in that direction, with a short walking animation. A footstep sound plays with every step.

Your character is the **blue sprite**. The direction you face matters — interactions only work with the NPC directly in front of you.

### 2. Find and talk to NPCs

Red squares on the map are NPCs. Walk up next to one (on an adjacent tile, facing them) and press **Space** to interact.

A dialog box slides up from the bottom of the screen with a **typewriter effect** — text appears character by character, Pokémon-style. Once the text finishes, a blinking triangle appears. Press **Space** again to dismiss the dialog.

### 3. Enter battle

If the NPC challenges you, these things happen in sequence:

1. The NPC glows and the screen **flashes white** — a dramatic Pokémon-style transition.
2. A **battle overlay** appears with a trivia question and a text input.
3. Type your answer and press **Enter** (or click the **Answer** button).
4. The outcome is shown for two seconds:
   - **Victory** — a triumphant arpeggio plays.
   - **Defeat** — a descending two-note tone plays.
5. You return to the map at your last position.

There is no penalty for losing — you can walk up to the NPC and challenge them again.

---

## Sound

The game uses the Web Audio API to generate chiptune-style sounds — no audio files needed.

| Sound | When |
|---|---|
| Background music | Starts ~1.5 seconds after the map loads. A looping 8-note square-wave melody. |
| Footstep | Every time you move one tile. |
| Battle start | When an NPC challenges you (rising square wave sweep). |
| Victory jingle | Ascending four-note arpeggio (C5 → E5 → G5 → C6). |
| Defeat tone | Descending two-note sequence (E4 → C4). |

Sounds play only after the first user interaction (browser autoplay policy). If your browser blocks audio, click anywhere on the page to unlock it.

---

## Offline Mode

If the backend API is unreachable when the game starts, it falls back to offline mode:

- A single NPC named **Sphinx** stands at position (2, 0).
- You can walk around the map, but **Space to interact does nothing** in offline mode.
- All map movement and animations still work.

---

## Tips

- The map boundary stops you at the edges — if your character doesn't move, you are probably at a wall or the map border.
- You can only move one tile at a time. Wait for the walking animation to finish before the next step.
- Facing direction counts when interacting with NPCs. Stand next to them on an adjacent tile and make sure you're facing toward them.
- In battle, press **Space** during a dialog's typewriter effect to skip straight to the end.
