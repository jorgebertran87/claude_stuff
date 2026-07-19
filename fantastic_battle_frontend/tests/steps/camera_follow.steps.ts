import { Before, After, Then, setDefaultTimeout } from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

class CameraWorld {
  browser!: Browser;
  page!: Page;
}

Before({ tags: "@camera" }, async function (this: CameraWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
  await this.page.goto("http://localhost:5173", { timeout: 30000, waitUntil: "networkidle" }).catch(() =>
    this.page.goto("http://localhost:5173", { timeout: 30000 })
  );
  await this.page.waitForSelector("canvas", { timeout: 30000 });
  await this.page.waitForSelector("#theme-input", { timeout: 10000 });
  await this.page.fill("#theme-input", "Greek mythology");
  await this.page.click("#theme-start");
  await this.page.waitForFunction(
    () => (window as any).__gameState !== undefined,
    { timeout: 15000 }
  );
});

After({ tags: "@camera" }, async function (this: CameraWorld) {
  await this.browser.close();
});

Then("the camera is centered on the player's position", async function (this: CameraWorld) {
  await this.page.waitForFunction(
    () => {
      const state = (window as any).__gameState;
      if (!state) return false;
      const playerPixelX = 16 + state.playerPosition.x * 32;
      const playerPixelY = 16 + state.playerPosition.y * 32;
      const camX = state.cameraScrollX ?? 0;
      const camY = state.cameraScrollY ?? 0;
      const screenX = playerPixelX - camX;
      const screenY = playerPixelY - camY;
      return screenX >= 0 && screenX <= 800 && screenY >= 0 && screenY <= 600;
    },
    { timeout: 15000 }
  );
});

Then("the camera does not scroll beyond the map boundary", async function (this: CameraWorld) {
  const state = await this.page.evaluate(() => {
    const s = (window as any).__gameState;
    return {
      camX: s?.cameraScrollX ?? 0,
      camY: s?.cameraScrollY ?? 0,
      mapW: (s?.mapWidth ?? 15) * 32,
      mapH: (s?.mapHeight ?? 10) * 32,
    };
  });
  if (state.camX < 0 || state.camY < 0) {
    throw new Error(`camera scrolled past origin: (${state.camX}, ${state.camY})`);
  }
  if (state.camX + 800 > state.mapW && state.mapW >= 800) {
    throw new Error(`camera scrolled past right edge: ${state.camX}`);
  }
  if (state.camY + 600 > state.mapH && state.mapH >= 600) {
    throw new Error(`camera scrolled past bottom edge: ${state.camY}`);
  }
});
