import { Before, After, Given, When, Then, setWorldConstructor, setDefaultTimeout } from "@cucumber/cucumber";
import { chromium, Browser, Page } from "playwright";

setDefaultTimeout(30000);

const NAV_PATHS: Record<string, string[]> = {
  "0,0": [],
  "1,0": ["east"],
  "1,1": ["east", "south"],
};

class GameWorld {
  browser!: Browser;
  page!: Page;
}

setWorldConstructor(GameWorld);

Before({ tags: "@map" }, async function (this: GameWorld) {
  this.browser = await chromium.launch({ headless: true });
  this.page = await this.browser.newPage();
});

After({ tags: "@map" }, async function (this: GameWorld) {
  await this.browser.close();
});

async function waitForGameState(page: Page) {
  await page.waitForFunction(
    () => (window as any).__gameState !== undefined,
    { timeout: 15000 }
  );
}

async function waitForNotMoving(page: Page) {
  await page.waitForFunction(
    () => {
      const state = (window as any).__gameState;
      return state && !state.isMoving;
    },
    { timeout: 15000 }
  );
}

async function debugMove(page: Page, direction: string) {
  await page.evaluate((dir: string) => {
    (window as any).__gameState.debugMove(dir);
  }, direction);
  await waitForNotMoving(page);
}

async function ensureAtPosition(page: Page, x: number, y: number) {
  const key = `${x},${y}`;
  const moves = NAV_PATHS[key];
  if (!moves) {
    throw new Error(`no navigation path to (${x}, ${y})`);
  }
  for (const direction of moves) {
    await debugMove(page, direction);
  }
}

async function loadGamePage(page: Page) {
  try {
    await page.goto("http://localhost:5173", { timeout: 30000, waitUntil: "networkidle" });
  } catch {
    await page.goto("http://localhost:5173", { timeout: 30000 });
  }
  await page.waitForSelector("canvas", { timeout: 30000 });
  await waitForGameState(page);
}

Given("the map has loaded", async function (this: GameWorld) {
  await loadGamePage(this.page);
});

Given("the player navigates to grid position \\({int}, {int}\\)", async function (this: GameWorld, x: number, y: number) {
  await loadGamePage(this.page);
  await ensureAtPosition(this.page, x, y);
});

When("I look at the game world", async function (this: GameWorld) {
  // no-op: the map is already rendered
});

When("the player enters the game", async function (this: GameWorld) {
  // no-op: player spawns automatically when the map loads
});

When("the player moves {word}", async function (this: GameWorld, direction: string) {
  await debugMove(this.page, direction.toLowerCase());
});

Then("the map is {int} tiles wide and {int} tiles tall", async function (this: GameWorld, width: number, height: number) {
  await this.page.waitForFunction(
    ([w, h]: [number, number]) => {
      const state = (window as any).__gameState;
      return state && state.mapWidth === w && state.mapHeight === h;
    },
    [width, height],
    { timeout: 15000 }
  );
});

Then("the player is at grid position \\({int}, {int}\\)", async function (this: GameWorld, x: number, y: number) {
  await this.page.waitForFunction(
    ([px, py]: [number, number]) => {
      const state = (window as any).__gameState;
      return state && state.playerPosition.x === px && state.playerPosition.y === py;
    },
    [x, y],
    { timeout: 15000 }
  );
});

Then("the player stays at grid position \\({int}, {int}\\)", async function (this: GameWorld, x: number, y: number) {
  await this.page.waitForTimeout(300);
  const pos = await this.page.evaluate(() => {
    const state = (window as any).__gameState;
    return state ? state.playerPosition : null;
  });
  if (!pos || pos.x !== x || pos.y !== y) {
    throw new Error(`expected player at (${x}, ${y}) but got (${pos?.x}, ${pos?.y})`);
  }
});

Then("the player is facing {word}", async function (this: GameWorld, direction: string) {
  await this.page.waitForFunction(
    (dir: string) => {
      const state = (window as any).__gameState;
      return state && state.playerDirection.toLowerCase() === dir.toLowerCase();
    },
    direction,
    { timeout: 15000 }
  );
});

Then("an NPC named {word} is at grid position \\({int}, {int}\\)", async function (this: GameWorld, name: string, x: number, y: number) {
  await this.page.waitForFunction(
    ([n, px, py]: [string, number, number]) => {
      const state = (window as any).__gameState;
      return state?.npcs?.some(
        (npc: any) => npc.name === n && npc.position.x === px && npc.position.y === py
      );
    },
    [name, x, y],
    { timeout: 15000 }
  );
});

Then("the NPC {word} is facing {word}", async function (this: GameWorld, name: string, direction: string) {
  await this.page.waitForFunction(
    ([n, dir]: [string, string]) => {
      const state = (window as any).__gameState;
      return state?.npcs?.some(
        (npc: any) =>
          npc.name === n && npc.direction.toLowerCase() === dir.toLowerCase()
      );
    },
    [name, direction],
    { timeout: 15000 }
  );
});
