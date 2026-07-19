import Phaser from "phaser";

const TILE_SIZE = 32;

const ACTIVE_COLOR = 0xff4444;
const CORRECT_COLOR = 0x4444ff;
const INCORRECT_COLOR = 0xcc2222;

export class Npc {
  private scene: Phaser.Scene;
  private name: string;
  private gridX: number;
  private gridY: number;
  private direction: string;
  private sprite: Phaser.GameObjects.Rectangle;
  private status: string;

  constructor(
    scene: Phaser.Scene,
    name: string,
    gridX: number,
    gridY: number,
    direction: string,
    status: string = "Active"
  ) {
    this.scene = scene;
    this.name = name;
    this.gridX = gridX;
    this.gridY = gridY;
    this.direction = direction;
    this.status = status;

    const pixelX = gridX * TILE_SIZE + TILE_SIZE / 2;
    const pixelY = gridY * TILE_SIZE + TILE_SIZE / 2;
    const color = Npc.statusColor(status);

    this.sprite = scene.add.rectangle(
      pixelX,
      pixelY,
      TILE_SIZE * 0.7,
      TILE_SIZE * 0.7,
      color
    );
    this.sprite.setDepth(10);
  }

  private static statusColor(status: string): number {
    switch (status) {
      case "DefeatedCorrect":
        return CORRECT_COLOR;
      case "DefeatedIncorrect":
        return INCORRECT_COLOR;
      default:
        return ACTIVE_COLOR;
    }
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

  getSprite(): Phaser.GameObjects.Rectangle {
    return this.sprite;
  }

  getStatus(): string {
    return this.status;
  }

  setStatus(status: string): void {
    this.status = status;
    this.sprite.setFillStyle(Npc.statusColor(status));
  }
}
