import Phaser from "phaser";
import { BootScene } from "./scenes/BootScene";
import { MapScene } from "./scenes/MapScene";

const config: Phaser.Types.Core.GameConfig = {
  type: Phaser.CANVAS,
  width: 800,
  height: 600,
  backgroundColor: "#2d2d2d",
  scene: [BootScene, MapScene],
};

new Phaser.Game(config);
