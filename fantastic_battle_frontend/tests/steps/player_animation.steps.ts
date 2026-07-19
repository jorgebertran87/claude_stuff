import { Before, After, When, Then, setDefaultTimeout } from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

const IDLE_FRAMES: Record<string, number> = {
  north: 3,
  south: 0,
  east: 6,
  west: 9,
};

class AnimationWorld {
  browser!: Browser;
  page!: Page;
}

Before({ tags: "@animation" }, async function (this: AnimationWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
  await this.page.goto("http://localhost:5173", { timeout: 30000, waitUntil: "networkidle" }).catch(() =>
    this.page.goto("http://localhost:5173", { timeout: 30000 })
  );
  await this.page.waitForSelector("canvas", { timeout: 30000 });
  await this.page.waitForFunction(
    () => (window as any).__gameState !== undefined,
    { timeout: 15000 }
  );
});

After({ tags: "@animation" }, async function (this: AnimationWorld) {
  await this.browser.close();
});

When("the player begins moving {word}", async function (this: AnimationWorld, direction: string) {
  await this.page.evaluate((dir: string) => {
    void (window as any).__gameState.debugMove(dir);
  }, direction.toLowerCase());
});

Then("the player shows a {word}-facing idle frame", async function (this: AnimationWorld, direction: string) {
  const expectedFrame = IDLE_FRAMES[direction.toLowerCase()];
  if (expectedFrame === undefined) {
    throw new Error(`unknown direction: ${direction}`);
  }
  await this.page.waitForFunction(
    (expected: number) => {
      const state = (window as any).__gameState;
      return state && !state.isMoving && state.playerFrame === expected;
    },
    expectedFrame,
    { timeout: 15000 }
  );
});

Then("the player is not moving", async function (this: AnimationWorld) {
  await this.page.waitForFunction(
    () => {
      const state = (window as any).__gameState;
      return state && !state.isMoving;
    },
    { timeout: 15000 }
  );
});

Then("the player changes to at least 2 different frames during the move", async function (this: AnimationWorld) {
  await this.page.waitForFunction(
    () => {
      const state = (window as any).__gameState;
      return state && state.isMoving;
    },
    { timeout: 5000 }
  );

  const seen = new Set<number>();
  const start = Date.now();
  while (Date.now() - start < 5000) {
    const frame = await this.page.evaluate(() => {
      return (window as any).__gameState?.playerFrame ?? -1;
    });
    if (frame >= 0) {
      seen.add(frame);
    }
    const moving = await this.page.evaluate(() => {
      return (window as any).__gameState?.isMoving ?? false;
    });
    if (!moving) {
      break;
    }
    await this.page.waitForTimeout(20);
  }
  if (seen.size < 2) {
    throw new Error(`expected at least 2 different frames but only saw: ${[...seen].join(", ")}`);
  }
});

Then("the player returns to an idle frame after movement completes", async function (this: AnimationWorld) {
  await this.page.waitForFunction(
    () => {
      const state = (window as any).__gameState;
      return state && !state.isMoving;
    },
    { timeout: 15000 }
  );
  const frame = await this.page.evaluate(() => {
    return (window as any).__gameState.playerFrame;
  });
  const idleFrames = [0, 3, 6, 9];
  if (!idleFrames.includes(frame)) {
    throw new Error(`expected idle frame (0, 3, 6, or 9) but got ${frame}`);
  }
});
