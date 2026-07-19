import Phaser from "phaser";

const CELL = 32;

export function generatePlayerTexture(scene: Phaser.Scene): string {
  const key = "player_sprite";
  if (scene.textures.exists(key)) {
    return key;
  }

  const width = CELL * 3;
  const height = CELL * 4;
  const canvas = scene.textures.createCanvas(key, width, height);
  if (!canvas) {
    throw new Error("failed to create player sprite texture");
  }

  const ctx = canvas.getContext();
  const bodyColor = "#4488ff";
  const headColor = "#66aaff";
  const bodyW = 14;
  const bodyH = 16;
  const headW = 10;
  const headH = 10;

  for (let dir = 0; dir < 4; dir++) {
    for (let frame = 0; frame < 3; frame++) {
      const ox = frame * CELL;
      const oy = dir * CELL;
      let bodyYOffset = 0;

      if (frame === 1) {
        bodyYOffset = -2;
      } else if (frame === 2) {
        bodyYOffset = 2;
      }

      const bodyX = ox + (CELL - bodyW) / 2;
      const bodyY = oy + CELL - bodyH - 2 + bodyYOffset;

      if (dir === 1) {
        ctx.fillStyle = headColor;
        ctx.fillRect(bodyX + (bodyW - headW) / 2, bodyY - headH + 4, headW, headH);
      }

      ctx.fillStyle = bodyColor;
      ctx.fillRect(bodyX, bodyY, bodyW, bodyH);

      if (dir !== 1) {
        ctx.fillStyle = headColor;
        ctx.fillRect(bodyX + (bodyW - headW) / 2, bodyY - headH + 2, headW, headH);
      }

      if (dir === 2 || dir === 3) {
        const legOffset = frame === 1 ? -1 : frame === 2 ? 1 : 0;
        ctx.fillStyle = bodyColor;
        const legY = bodyY + bodyH;
        if (dir === 2) {
          ctx.fillRect(bodyX + 2 + legOffset, legY, 4, 4);
          ctx.fillRect(bodyX + bodyW - 6 - legOffset, legY, 4, 4);
        } else {
          ctx.fillRect(bodyX + 2 + legOffset, legY, 4, 4);
          ctx.fillRect(bodyX + bodyW - 6 + legOffset, legY, 4, 4);
        }
      }
    }
  }

  canvas.refresh();

  const texture = scene.textures.get(key);
  for (let dir = 0; dir < 4; dir++) {
    for (let frame = 0; frame < 3; frame++) {
      const index = dir * 3 + frame;
      texture.add(index, 0, frame * CELL, dir * CELL, CELL, CELL);
    }
  }

  return key;
}
