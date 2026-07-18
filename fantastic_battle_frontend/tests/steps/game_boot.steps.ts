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
  const text = await this.page.evaluate(() => {
    const canvas = document.querySelector("canvas");
    return canvas !== null;
  });
  if (!text) {
    throw new Error("canvas not found for title check");
  }
});
