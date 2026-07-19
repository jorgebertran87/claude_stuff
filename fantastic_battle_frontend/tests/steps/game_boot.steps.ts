import { Before, After, Given, When, Then, setWorldConstructor } from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

class GameWorld {
  browser!: Browser;
  page!: Page;
}

setWorldConstructor(GameWorld);

Before({ tags: "@boot" }, async function (this: GameWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@boot" }, async function (this: GameWorld) {
  await this.browser.close();
});

When("the game starts", async function (this: GameWorld) {
  await this.page.goto("http://localhost:5173");
  await this.page.waitForSelector("canvas");
});

Then("the game canvas is visible", async function (this: GameWorld) {
  const canvas = await this.page.$("canvas");
  if (!canvas) {
    throw new Error("canvas not found");
  }
});

Then("the title {string} is displayed", async function (this: GameWorld, title: string) {
  const titleEl = await this.page.$("#theme-prompt h1");
  if (!titleEl) {
    throw new Error("theme prompt title not found");
  }
  const text = await titleEl.textContent();
  if (!text || !text.includes(title)) {
    throw new Error(`expected title "${title}" but got "${text}"`);
  }
});
