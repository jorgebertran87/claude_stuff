import { Before, After, When, Then, setDefaultTimeout } from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

class DialogWorld {
  browser!: Browser;
  page!: Page;
  textLengths: number[] = [];
}

Before({ tags: "@dialog" }, async function (this: DialogWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@dialog" }, async function (this: DialogWorld) {
  await this.browser.close();
});

When("the player triggers an NPC interaction", async function (this: DialogWorld) {
  this.textLengths = [];
  await this.page.click("canvas");
  await this.page.waitForTimeout(100);
  await this.page.keyboard.down("Space");
  await this.page.waitForTimeout(100);
  await this.page.keyboard.up("Space");

  await this.page.waitForFunction(
    () => {
      const ds = (window as any).__dialogState;
      return ds !== undefined && ds !== null;
    },
    { timeout: 15000 }
  );

  const start = Date.now();
  while (Date.now() - start < 15000) {
    const [len, done] = await this.page.evaluate(() => {
      const ds = (window as any).__dialogState;
      return [ds?.displayedLength ?? 0, ds?.typewriterComplete ?? false];
    });
    if (len > 0 && (this.textLengths.length === 0 || len !== this.textLengths[this.textLengths.length - 1])) {
      this.textLengths.push(len);
    }
    if (done) {
      break;
    }
    await this.page.waitForTimeout(15);
  }
});

Then("a dialog box is visible on screen", async function (this: DialogWorld) {
  await this.page.waitForFunction(
    () => {
      const ds = (window as any).__dialogState;
      return ds && ds.visible;
    },
    { timeout: 15000 }
  );
});

Then("the dialog box text grows character by character", async function (this: DialogWorld) {
  const lengths = this.textLengths;
  if (lengths.length < 2) {
    throw new Error(`expected growing text but only saw: ${lengths.join(", ")}`);
  }
  const increasing = lengths.every((v, i) => i === 0 || v >= lengths[i - 1]);
  if (!increasing) {
    throw new Error(`text length did not grow monotonically: ${lengths.join(", ")}`);
  }
});

Then("the dialog text has finished appearing", async function (this: DialogWorld) {
  await this.page.waitForFunction(
    () => {
      const ds = (window as any).__dialogState;
      return ds && ds.typewriterComplete;
    },
    { timeout: 15000 }
  );
});

When("the player presses space", async function (this: DialogWorld) {
  await this.page.keyboard.down("Space");
  await this.page.waitForTimeout(150);
  await this.page.keyboard.up("Space");
  await this.page.waitForTimeout(300);
});

Then("the dialog box is dismissed", async function (this: DialogWorld) {
  await this.page.waitForFunction(
    () => {
      const ds = (window as any).__dialogState;
      return !ds || !ds.visible;
    },
    { timeout: 15000 }
  );
});
