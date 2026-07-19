import {
  Before,
  After,
  When,
  Then,
  setWorldConstructor,
  setDefaultTimeout,
} from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

class BattleWorld {
  browser!: Browser;
  page!: Page;
}

setWorldConstructor(BattleWorld);

Before({ tags: "@battle" }, async function (this: BattleWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@battle" }, async function (this: BattleWorld) {
  await this.browser.close();
});

When(
  "the player presses space to interact",
  async function (this: BattleWorld) {
    await this.page.click("canvas");
    await this.page.waitForTimeout(100);
    await this.page.keyboard.down("Space");
    await this.page.waitForTimeout(100);
    await this.page.keyboard.up("Space");
    await this.page.waitForTimeout(500);
  }
);

async function checkBattleQuestionDisplayed(page: Page) {
  await page.waitForSelector("#battle-overlay", { timeout: 10000 });
  const question = await page.textContent("#battle-question");
  if (!question || question.trim().length === 0) {
    throw new Error("expected battle question but got empty or missing text");
  }
}

Then("a battle question is displayed", async function (this: BattleWorld) {
  await checkBattleQuestionDisplayed(this.page);
});

When("the player answers {string}", async function (this: BattleWorld, answer: string) {
  await this.page.waitForSelector("#battle-answer-input", { timeout: 5000 });
  await this.page.fill("#battle-answer-input", answer);
  await this.page.click("#battle-submit");
  await this.page.waitForTimeout(500);
});

Then(
  "the battle outcome {string} is displayed",
  async function (this: BattleWorld, outcome: string) {
    await this.page.waitForSelector("#battle-outcome", { timeout: 10000 });
    const text = await this.page.textContent("#battle-outcome");
    if (!text || !text.includes(outcome)) {
      throw new Error(
        `expected outcome "${outcome}" but got "${text}"`
      );
    }
  }
);
