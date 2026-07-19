import { Before, After, Then, setDefaultTimeout } from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

class TransitionWorld {
  browser!: Browser;
  page!: Page;
}

Before({ tags: "@transition" }, async function (this: TransitionWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@transition" }, async function (this: TransitionWorld) {
  await this.browser.close();
});

Then("a flash overlay appears before the battle starts", async function (this: TransitionWorld) {
  await this.page.waitForFunction(
    () => {
      return (window as any).__flashWasActive === true;
    },
    { timeout: 15000 }
  );
});

Then("the interacted NPC shows a glow effect", async function (this: TransitionWorld) {
  await this.page.waitForFunction(
    () => {
      const name = (window as any).__npcGlowed;
      return typeof name === "string" && name.length > 0;
    },
    { timeout: 15000 }
  );
});
