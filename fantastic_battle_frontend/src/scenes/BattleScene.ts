import Phaser from "phaser";
import { ApiClient } from "../services/ApiClient";
import { BattleOverlay } from "../ui/BattleOverlay";

export class BattleScene extends Phaser.Scene {
  constructor() {
    super({ key: "BattleScene" });
  }

  async create(): Promise<void> {
    const { question } = this.scene.settings.data as {
      npcName: string;
      question: string;
      sessionId: string;
    };
    const apiClient = this.game.registry.get("apiClient") as ApiClient;
    const overlay = new BattleOverlay();

    const answer = await overlay.show(question);
    const response = await apiClient.answer(answer);
    await overlay.showOutcome(response.outcome);

    this.scene.start("MapScene");
  }
}
