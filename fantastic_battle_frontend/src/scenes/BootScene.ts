import Phaser from "phaser";
import { ThemePrompt } from "../ui/ThemePrompt";

export class BootScene extends Phaser.Scene {
  constructor() {
    super({ key: "BootScene" });
  }

  async create(): Promise<void> {
    const prompt = new ThemePrompt();
    const result = await prompt.show();
    prompt.remove();

    this.game.registry.set("theme", result.theme);
    this.game.registry.set("questionCount", result.questionCount);

    this.scene.start("MapScene");
  }
}
