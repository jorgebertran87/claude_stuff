import Phaser from "phaser";
import { Player } from "../game/Player";
import { Npc } from "../game/Npc";
import { GameState } from "../game/GameState";
import { ApiClient } from "../services/ApiClient";

const TILE_SIZE = 32;
const MAP_WIDTH_TILES = 5;
const MAP_HEIGHT_TILES = 5;
const MAP_PIXEL_W = MAP_WIDTH_TILES * TILE_SIZE;
const MAP_PIXEL_H = MAP_HEIGHT_TILES * TILE_SIZE;

const DIRECTION_DELTA: Record<string, { dx: number; dy: number }> = {
  north: { dx: 0, dy: -1 },
  south: { dx: 0, dy: 1 },
  east: { dx: 1, dy: 0 },
  west: { dx: -1, dy: 0 },
};

const CURSOR_TO_DIRECTION: Record<string, string> = {
  up: "north",
  down: "south",
  left: "west",
  right: "east",
};

export class MapScene extends Phaser.Scene {
  private player!: Player;
  private npcs: Npc[] = [];
  private cursors!: Phaser.Types.Input.Keyboard.CursorKeys;
  private groundLayer!: Phaser.Tilemaps.TilemapLayer;
  private offsetX = 0;
  private offsetY = 0;
  private lastDirection: string | null = null;
  private apiClient!: ApiClient;
  private lastSpaceState = false;
  private hasApiSession = false;

  constructor() {
    super({ key: "MapScene" });
  }

  preload(): void {
    const canvasTexture = this.textures.createCanvas(
      "tiles",
      TILE_SIZE * 2,
      TILE_SIZE
    );
    if (!canvasTexture) {
      throw new Error("failed to create tileset texture");
    }
    const ctx = canvasTexture.getContext();
    ctx.fillStyle = "#4caf50";
    ctx.fillRect(0, 0, TILE_SIZE, TILE_SIZE);
    ctx.fillStyle = "#9e9e9e";
    ctx.fillRect(TILE_SIZE, 0, TILE_SIZE, TILE_SIZE);
    canvasTexture.refresh();

    this.load.tilemapTiledJSON("map", "assets/map.json");
  }

  async create(): Promise<void> {
    this.apiClient = this.game.registry.get("apiClient");

    this.offsetX = (800 - MAP_PIXEL_W) / 2;
    this.offsetY = (600 - MAP_PIXEL_H) / 2;

    const map = this.make.tilemap({ key: "map" });
    const tileset = map.addTilesetImage("tiles", "tiles");
    if (!tileset) {
      throw new Error("tileset not found");
    }
    this.groundLayer = map.createLayer("ground", tileset)!;
    this.groundLayer.setPosition(this.offsetX, this.offsetY);

    try {
      const session = await this.apiClient.join();
      this.hasApiSession = true;

      this.player = new Player(
        this,
        session.player_position.x,
        session.player_position.y,
        session.player_direction,
        this.offsetX,
        this.offsetY
      );

      this.npcs = session.npcs.map(
        (npcData) =>
          new Npc(
            this,
            npcData.name,
            npcData.position.x,
            npcData.position.y,
            npcData.direction,
            this.offsetX,
            this.offsetY
          )
      );
    } catch {
      this.hasApiSession = false;

      this.player = new Player(
        this,
        0,
        0,
        "south",
        this.offsetX,
        this.offsetY
      );

      this.npcs = [
        new Npc(this, "Sphinx", 2, 0, "south", this.offsetX, this.offsetY),
      ];
    }

    this.cursors = this.input.keyboard!.createCursorKeys();

    this.exposeGameState();
  }

  update(): void {
    if (!this.player || !this.cursors) {
      return;
    }
    if (this.player.getIsMoving()) {
      this.exposeGameState();
      return;
    }

    for (const [cursorKey, direction] of Object.entries(CURSOR_TO_DIRECTION)) {
      const key = (this.cursors as any)[cursorKey] as Phaser.Input.Keyboard.Key;
      if (key && key.isDown) {
        if (this.lastDirection !== direction) {
          this.lastDirection = direction;
          this.tryMove(direction);
        }
        break;
      }
      if (this.lastDirection === direction) {
        this.lastDirection = null;
      }
    }

    const spaceDown = this.cursors.space.isDown;
    if (spaceDown && !this.lastSpaceState) {
      this.tryInteract();
    }
    this.lastSpaceState = spaceDown;

    this.exposeGameState();
  }

  private async tryMove(direction: string): Promise<void> {
    if (this.hasApiSession) {
      try {
        const response = await this.apiClient.move(direction);
        this.player.animateTo(
          response.player_position.x,
          response.player_position.y,
          response.player_direction,
          () => this.exposeGameState()
        );
        this.exposeGameState();
      } catch {
        this.exposeGameState();
      }
      return;
    }

    const delta = DIRECTION_DELTA[direction];
    const pos = this.player.getGridPosition();
    const targetX = pos.x + delta.dx;
    const targetY = pos.y + delta.dy;

    if (!this.isWalkable(targetX, targetY)) {
      this.exposeGameState();
      return;
    }

    this.player.animateTo(targetX, targetY, direction, () =>
      this.exposeGameState()
    );
    this.exposeGameState();
  }

  private isWalkable(gridX: number, gridY: number): boolean {
    if (
      gridX < 0 ||
      gridX >= MAP_WIDTH_TILES ||
      gridY < 0 ||
      gridY >= MAP_HEIGHT_TILES
    ) {
      return false;
    }
    const tile = this.groundLayer.getTileAt(gridX, gridY);
    if (!tile) {
      return false;
    }
    return tile.index !== 2;
  }

  private async tryInteract(): Promise<void> {
    if (!this.hasApiSession) {
      return;
    }
    try {
      const response = await this.apiClient.interact();
      if (response.battle) {
        this.scene.start("BattleScene", {
          npcName: response.npc?.name ?? "",
          question: response.battle.question,
          sessionId: this.apiClient.getSessionId(),
        });
        return;
      }
      this.exposeGameState();
    } catch {
      this.exposeGameState();
    }
  }

  private exposeGameState(): void {
    const state: GameState = {
      playerPosition: this.player.getGridPosition(),
      playerDirection: this.player.getDirection(),
      isMoving: this.player.getIsMoving(),
      mapWidth: MAP_WIDTH_TILES,
      mapHeight: MAP_HEIGHT_TILES,
      npcs: this.npcs.map((npc) => ({
        name: npc.getName(),
        position: npc.getGridPosition(),
        direction: npc.getDirection(),
      })),
    };

    const self = this;
    (window as any).__gameState = Object.assign(state, {
      debugMove: (direction: string): Promise<void> => {
        return self.tryMove(direction);
      },
      debugInteract: (): Promise<void> => {
        return self.tryInteract();
      },
      debugHasApi: () => self.hasApiSession,
    });
  }
}
