import Phaser from "phaser";
import { ThemePrompt } from "../ui/ThemePrompt";

export class BootScene extends Phaser.Scene {
  constructor() {
    super({ key: "BootScene" });
  }

  async create(): Promise<void> {
    const prompt = new ThemePrompt();
    const theme = await prompt.show();
    prompt.remove();

    this.game.registry.set("theme", theme);

    this.scene.start("MapScene");
  }
}
