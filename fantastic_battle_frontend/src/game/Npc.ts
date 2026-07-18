import Phaser from "phaser";

const TILE_SIZE = 32;

export class Npc {
  private scene: Phaser.Scene;
  private name: string;
  private gridX: number;
  private gridY: number;
  private direction: string;
  private sprite: Phaser.GameObjects.Rectangle;

  constructor(
    scene: Phaser.Scene,
    name: string,
    gridX: number,
    gridY: number,
    direction: string,
    offsetX: number,
    offsetY: number
  ) {
    this.scene = scene;
    this.name = name;
    this.gridX = gridX;
    this.gridY = gridY;
    this.direction = direction;

    const pixelX = offsetX + gridX * TILE_SIZE + TILE_SIZE / 2;
    const pixelY = offsetY + gridY * TILE_SIZE + TILE_SIZE / 2;

    this.sprite = scene.add.rectangle(
      pixelX,
      pixelY,
      TILE_SIZE * 0.7,
      TILE_SIZE * 0.7,
      0xff4444
    );
    this.sprite.setDepth(10);
  }

  getName(): string {
    return this.name;
  }

  getGridPosition(): { x: number; y: number } {
    return { x: this.gridX, y: this.gridY };
  }

  getDirection(): string {
    return this.direction;
  }
}
