import Phaser from "phaser";
import { BootScene } from "./scenes/BootScene";
import { MapScene } from "./scenes/MapScene";
import { BattleScene } from "./scenes/BattleScene";
import { ApiClient } from "./services/ApiClient";
import { SoundService } from "./services/SoundService";

const apiClient = new ApiClient("http://localhost:8081");
const soundService = new SoundService();

const config: Phaser.Types.Core.GameConfig = {
  type: Phaser.CANVAS,
  width: 800,
  height: 600,
  backgroundColor: "#2d2d2d",
  scene: [BootScene, MapScene, BattleScene],
  callbacks: {
    preBoot: (game: Phaser.Game) => {
      game.registry.set("apiClient", apiClient);
      game.registry.set("soundService", soundService);
    },
  },
};

new Phaser.Game(config);
