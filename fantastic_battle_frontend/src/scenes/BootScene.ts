import Phaser from "phaser";

export class BootScene extends Phaser.Scene {
  constructor() {
    super({ key: "BootScene" });
  }

  create(): void {
    this.add.text(400, 300, "Fantastic Battle", {
      fontSize: "32px",
      color: "#ffffff",
    }).setOrigin(0.5);

    this.time.delayedCall(200, () => {
      this.scene.start("MapScene");
    });
  }
}
