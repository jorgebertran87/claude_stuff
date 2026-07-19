import { Before, After, Then, setDefaultTimeout } from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

class SoundWorld {
  browser!: Browser;
  page!: Page;
}

Before({ tags: "@sound" }, async function (this: SoundWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@sound" }, async function (this: SoundWorld) {
  await this.browser.close();
});

Then("background music begins playing", async function (this: SoundWorld) {
  await this.page.waitForFunction(
    () => {
      const ss = (window as any).__soundState;
      return ss && ss.bgmPlaying === true;
    },
    { timeout: 15000 }
  );
});

Then("a footstep sound is triggered", async function (this: SoundWorld) {
  await this.page.waitForFunction(
    () => {
      const ss = (window as any).__soundState;
      return ss && ss.lastEffectPlayed === "footstep";
    },
    { timeout: 15000 }
  );
});

Then("a battle start sound is triggered", async function (this: SoundWorld) {
  await this.page.waitForFunction(
    () => {
      const ss = (window as any).__soundState;
      return ss && ss.lastEffectPlayed === "battleStart";
    },
    { timeout: 15000 }
  );
});

Then("a victory sound is triggered", async function (this: SoundWorld) {
  await this.page.waitForFunction(
    () => {
      const ss = (window as any).__soundState;
      return ss && ss.lastEffectPlayed === "victory";
    },
    { timeout: 15000 }
  );
});

Then("a defeat sound is triggered", async function (this: SoundWorld) {
  await this.page.waitForFunction(
    () => {
      const ss = (window as any).__soundState;
      return ss && ss.lastEffectPlayed === "defeat";
    },
    { timeout: 15000 }
  );
});
