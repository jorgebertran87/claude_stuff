import {
  Before,
  After,
  Then,
  setWorldConstructor,
  setDefaultTimeout,
} from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

class E2eWorld {
  browser!: Browser;
  page!: Page;
}

setWorldConstructor(E2eWorld);

Before({ tags: "@e2e" }, async function (this: E2eWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@e2e" }, async function (this: E2eWorld) {
  await this.browser.close();
});

Then(
  "the player returns to the map at grid position \\({int}, {int}\\)",
  async function (this: E2eWorld, x: number, y: number) {
    await this.page.waitForSelector("canvas", { timeout: 10000 });
    await this.page.waitForFunction(
      ([px, py]: [number, number]) => {
        const state = (window as any).__gameState;
        return (
          state &&
          state.playerPosition &&
          state.playerPosition.x === px &&
          state.playerPosition.y === py &&
          !state.isMoving
        );
      },
      [x, y],
      { timeout: 15000 }
    );
  }
);
