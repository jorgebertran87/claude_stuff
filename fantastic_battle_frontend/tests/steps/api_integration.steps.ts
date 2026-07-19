import {
  Before,
  After,
  Given,
  setWorldConstructor,
  setDefaultTimeout,
} from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

class ApiWorld {
  browser!: Browser;
  page!: Page;
}

setWorldConstructor(ApiWorld);

Before({ tags: "@api" }, async function (this: ApiWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@api" }, async function (this: ApiWorld) {
  await this.browser.close();
});

Given("the game loads with the API backend", async function (this: ApiWorld) {
  try {
    await this.page.goto("http://localhost:5173", {
      timeout: 30000,
      waitUntil: "networkidle",
    });
  } catch {
    await this.page.goto("http://localhost:5173", { timeout: 30000 });
  }
  await this.page.waitForSelector("canvas", { timeout: 30000 });
  await this.page.waitForFunction(
    () => (window as any).__gameState !== undefined,
    { timeout: 15000 }
  );
});
