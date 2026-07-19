import Phaser from "phaser";
import { Player } from "../game/Player";
import { Npc } from "../game/Npc";
import { GameState } from "../game/GameState";
import { ApiClient } from "../services/ApiClient";
import { DialogBox } from "../ui/DialogBox";
import { SoundService } from "../services/SoundService";

const TILE_SIZE = 32;
const MAP_WIDTH_TILES = 15;
const MAP_HEIGHT_TILES = 10;
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

const GRASS_COLOR = 0x4caf50;
const WALL_COLOR = 0x9e9e9e;
const WALL_TILE_INDEX = 2;

export class MapScene extends Phaser.Scene {
  private player!: Player;
  private npcs: Npc[] = [];
  private cursors!: Phaser.Types.Input.Keyboard.CursorKeys;
  private tileData: number[] = [];
  private tileGraphics: Phaser.GameObjects.Rectangle[] = [];
  private lastDirection: string | null = null;
  private apiClient!: ApiClient;
  private lastSpaceState = false;
  private hasApiSession = false;
  private dialog: DialogBox | null = null;
  private soundService!: SoundService;

  constructor() {
    super({ key: "MapScene" });
  }

  preload(): void {
    this.load.json("map_data", "assets/map.json");
  }

  create(): void {
    this.apiClient = this.game.registry.get("apiClient");
    this.soundService = this.game.registry.get("soundService");

    this.renderTiles();

    this.cameras.main.setBounds(0, 0, MAP_PIXEL_W, MAP_PIXEL_H);
    this.cursors = this.input.keyboard!.createCursorKeys();

    this.initPlayerAndNpcs();

    if (this.soundService) {
      this.time.delayedCall(1500, () => {
        this.soundService.startBgm();
      });
    }

    this.exposeGameState();
  }

  private renderTiles(): void {
    const mapData = this.cache.json.get("map_data");
    if (!mapData) {
      return;
    }

    const groundLayer = mapData.layers?.find(
      (l: { name: string }) => l.name === "ground"
    );
    if (!groundLayer || !groundLayer.data) {
      return;
    }

    this.tileData = groundLayer.data;

    const viewCols = Math.ceil(800 / TILE_SIZE);
    const viewRows = Math.ceil(600 / TILE_SIZE);

    for (let row = 0; row < viewRows; row++) {
      for (let col = 0; col < viewCols; col++) {
        const inMap =
          col < MAP_WIDTH_TILES && row < MAP_HEIGHT_TILES;
        const mapIndex = row * MAP_WIDTH_TILES + col;

        let color: number;
        if (inMap) {
          const tileIndex = this.tileData[mapIndex];
          if (tileIndex === 0) {
            continue;
          }
          color = tileIndex === WALL_TILE_INDEX ? WALL_COLOR : GRASS_COLOR;
        } else {
          color = 0x333333;
        }

        const pixelX = col * TILE_SIZE + TILE_SIZE / 2;
        const pixelY = row * TILE_SIZE + TILE_SIZE / 2;

        const rect = this.add.rectangle(
          pixelX, pixelY, TILE_SIZE, TILE_SIZE, color
        );
        rect.setDepth(0);
        this.tileGraphics.push(rect);
      }
    }
  }

  private async initPlayerAndNpcs(): Promise<void> {
    try {
      const theme = this.game.registry.get("theme") as string | undefined;
      const session = await this.apiClient.join(theme);
      this.hasApiSession = true;

      this.player = new Player(
        this,
        session.player_position.x,
        session.player_position.y,
        session.player_direction
      );

      this.npcs = session.npcs.map(
        (npcData) =>
          new Npc(
            this,
            npcData.name,
            npcData.position.x,
            npcData.position.y,
            npcData.direction
          )
      );
    } catch {
      this.hasApiSession = false;
      this.player = new Player(this, 0, 0, "south");
      this.npcs = [new Npc(this, "Sphinx", 2, 0, "south")];
    }

    this.cameras.main.startFollow(this.player.getSprite(), true, 0.1, 0.1);
    this.exposeGameState();
  }

  update(): void {
    if (this.dialog) {
      this.dialog.update();
      this.exposeGameState();
      return;
    }

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
        if (this.soundService) {
          this.soundService.playFootstep();
        }
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

    if (this.soundService) {
      this.soundService.playFootstep();
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
    const index = gridY * MAP_WIDTH_TILES + gridX;
    if (index < 0 || index >= this.tileData.length) {
      return false;
    }
    return this.tileData[index] !== WALL_TILE_INDEX;
  }

  private async tryInteract(): Promise<void> {
    if (!this.hasApiSession) {
      return;
    }
    try {
      const response = await this.apiClient.interact();
      if (response.battle) {
        const npcName = response.npc?.name ?? "";
        if (this.soundService) {
          this.soundService.playBattleStart();
        }
        this.dialog = new DialogBox(this, `${npcName}: Prepare for battle!`);
        await this.dialog.show();
        this.dialog = null;
        const npcSprite = this.npcs
          .find((n) => n.getName() === npcName)
          ?.getSprite();
        await this.playBattleTransition(npcSprite ?? null, npcName);
        this.scene.start("BattleScene", {
          npcName,
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

  private playBattleTransition(
    npcSprite: Phaser.GameObjects.Rectangle | null,
    npcName: string
  ): Promise<void> {
    return new Promise((resolve) => {
      const flash = this.add.rectangle(400, 300, 800, 600, 0xffffff, 0);
      flash.setDepth(100);
      flash.setScrollFactor(0);

      this.exposeGameState();

      this.tweens.add({
        targets: flash,
        alpha: 0.8,
        duration: 100,
        yoyo: true,
        hold: 100,
        onStart: () => {
          this.updateTransitionState(true, npcName);
        },
        onComplete: () => {
          flash.destroy();
          this.updateTransitionState(false, null);
          resolve();
        },
      });

      if (npcSprite) {
        this.tweens.add({
          targets: npcSprite,
          scaleX: 1.3,
          scaleY: 1.3,
          duration: 150,
          yoyo: true,
        });
      }
    });
  }

  private updateTransitionState(active: boolean, target: string | null): void {
    const state = (window as any).__gameState;
    if (state) {
      state.transitionFlashActive = active;
      state.npcGlowTarget = target;
    }
    if (active) {
      (window as any).__flashWasActive = true;
      if (target) {
        (window as any).__npcGlowed = target;
      }
    }
  }

  private exposeGameState(): void {
    if (!this.player) {
      return;
    }
    const state: GameState = {
      playerPosition: this.player.getGridPosition(),
      playerDirection: this.player.getDirection(),
      isMoving: this.player.getIsMoving(),
      playerFrame: this.player.getFrame(),
      mapWidth: MAP_WIDTH_TILES,
      mapHeight: MAP_HEIGHT_TILES,
      npcs: this.npcs.map((npc) => ({
        name: npc.getName(),
        position: npc.getGridPosition(),
        direction: npc.getDirection(),
      })),
      cameraScrollX: this.cameras.main.scrollX,
      cameraScrollY: this.cameras.main.scrollY,
      transitionFlashActive: false,
      npcGlowTarget: null,
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
