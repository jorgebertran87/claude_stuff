import Phaser from "phaser";

const TILE_SIZE = 32;
const MOVE_DURATION = 150;

export class Player {
  private scene: Phaser.Scene;
  private sprite: Phaser.GameObjects.Rectangle;
  private gridX: number;
  private gridY: number;
  private direction: string;
  private moving: boolean;
  private offsetX: number;
  private offsetY: number;

  constructor(
    scene: Phaser.Scene,
    gridX: number,
    gridY: number,
    direction: string,
    offsetX: number,
    offsetY: number
  ) {
    this.scene = scene;
    this.gridX = gridX;
    this.gridY = gridY;
    this.direction = direction;
    this.moving = false;
    this.offsetX = offsetX;
    this.offsetY = offsetY;

    const pixelX = this.toPixelX(gridX);
    const pixelY = this.toPixelY(gridY);

    this.sprite = scene.add.rectangle(
      pixelX,
      pixelY,
      TILE_SIZE * 0.7,
      TILE_SIZE * 0.7,
      0x4488ff
    );
    this.sprite.setDepth(10);
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

    const pixelX = this.toPixelX(targetX);
    const pixelY = this.toPixelY(targetY);

    this.scene.tweens.add({
      targets: this.sprite,
      x: pixelX,
      y: pixelY,
      duration: MOVE_DURATION,
      onComplete: () => {
        this.moving = false;
        if (onComplete) {
          onComplete();
        }
      },
    });
  }

  private toPixelX(gridX: number): number {
    return this.offsetX + gridX * TILE_SIZE + TILE_SIZE / 2;
  }

  private toPixelY(gridY: number): number {
    return this.offsetY + gridY * TILE_SIZE + TILE_SIZE / 2;
  }
}
