import Phaser from "phaser";
import { generatePlayerTexture } from "./SpriteGenerator";

const TILE_SIZE = 32;
const MOVE_DURATION = 150;

const DIRECTION_ROW: Record<string, number> = {
  south: 0,
  north: 1,
  east: 2,
  west: 3,
};

export class Player {
  private scene: Phaser.Scene;
  private sprite: Phaser.GameObjects.Sprite;
  private gridX: number;
  private gridY: number;
  private direction: string;
  private moving: boolean;
  private frame: number;
  private walkTimer: Phaser.Time.TimerEvent | null;

  constructor(
    scene: Phaser.Scene,
    gridX: number,
    gridY: number,
    direction: string
  ) {
    this.scene = scene;
    this.gridX = gridX;
    this.gridY = gridY;
    this.direction = direction;
    this.moving = false;
    this.walkTimer = null;

    generatePlayerTexture(scene);

    const pixelX = gridX * TILE_SIZE + TILE_SIZE / 2;
    const pixelY = gridY * TILE_SIZE + TILE_SIZE / 2;

    this.sprite = scene.add.sprite(pixelX, pixelY, "player_sprite");
    this.sprite.setDepth(10);
    this.frame = this.idleFrame(direction);
    this.sprite.setFrame(this.frame);
  }

  getGridPosition(): { x: number; y: number } {
    return { x: this.gridX, y: this.gridY };
  }

  getDirection(): string {
    return this.direction;
  }

  getIsMoving(): boolean {
    return this.moving;
  }

  getFrame(): number {
    return this.frame;
  }

  getSprite(): Phaser.GameObjects.Sprite {
    return this.sprite;
  }

  animateTo(
    targetX: number,
    targetY: number,
    direction: string,
    onComplete?: () => void
  ): void {
    if (this.moving) {
      return;
    }
    this.gridX = targetX;
    this.gridY = targetY;
    this.direction = direction.toLowerCase();
    this.moving = true;

    const dirRow = DIRECTION_ROW[this.direction] ?? 0;
    this.setFrame(dirRow, 1);

    if (this.walkTimer) {
      this.walkTimer.destroy();
    }
    this.walkTimer = this.scene.time.delayedCall(MOVE_DURATION / 2, () => {
      this.setFrame(dirRow, 2);
    });

    const pixelX = targetX * TILE_SIZE + TILE_SIZE / 2;
    const pixelY = targetY * TILE_SIZE + TILE_SIZE / 2;

    this.scene.tweens.add({
      targets: this.sprite,
      x: pixelX,
      y: pixelY,
      duration: MOVE_DURATION,
      onComplete: () => {
        this.moving = false;
        this.setFrame(dirRow, 0);
        if (onComplete) {
          onComplete();
        }
      },
    });
  }

  private idleFrame(direction: string): number {
    const row = DIRECTION_ROW[direction] ?? 0;
    return row * 3;
  }

  private setFrame(directionRow: number, frameIndex: number): void {
    this.frame = directionRow * 3 + frameIndex;
    this.sprite.setFrame(this.frame);
  }
}
