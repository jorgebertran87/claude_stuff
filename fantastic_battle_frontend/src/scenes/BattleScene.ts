import Phaser from "phaser";
import { ApiClient } from "../services/ApiClient";
import { BattleOverlay } from "../ui/BattleOverlay";
import { SoundService } from "../services/SoundService";

export class BattleScene extends Phaser.Scene {
  constructor() {
    super({ key: "BattleScene" });
  }

  async create(): Promise<void> {
    const { npcName, question, sessionId } = this.scene.settings.data as {
      npcName: string;
      question: string;
      sessionId: string;
    };
    const apiClient = this.game.registry.get("apiClient") as ApiClient;
    const soundService = this.game.registry.get("soundService") as SoundService;
    const overlay = new BattleOverlay();

    const answer = await overlay.show(question);
    const response = await apiClient.answer(answer);

    if (soundService) {
      if (response.outcome === "Victory") {
        soundService.playVictory();
      } else {
        soundService.playDefeat();
      }
    }

    await overlay.showOutcome(response.outcome);

    this.scene.start("MapScene", {
      sessionId,
      npcName,
      outcome: response.outcome,
    });
  }
}
