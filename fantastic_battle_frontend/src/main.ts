import Phaser from "phaser";
import { BootScene } from "./scenes/BootScene";
import { MapScene } from "./scenes/MapScene";
import { BattleScene } from "./scenes/BattleScene";
import { ApiClient } from "./services/ApiClient";

const apiClient = new ApiClient("http://localhost:8081");

const config: Phaser.Types.Core.GameConfig = {
  type: Phaser.CANVAS,
  width: 800,
  height: 600,
  backgroundColor: "#2d2d2d",
  scene: [BootScene, MapScene, BattleScene],
  callbacks: {
    preBoot: (game: Phaser.Game) => {
      game.registry.set("apiClient", apiClient);
    },
  },
};

new Phaser.Game(config);
