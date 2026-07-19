export interface NpcState {
  name: string;
  position: { x: number; y: number };
  direction: string;
}

export interface GameState {
  playerPosition: { x: number; y: number };
  playerDirection: string;
  isMoving: boolean;
  playerFrame: number;
  mapWidth: number;
  mapHeight: number;
  npcs: NpcState[];
  cameraScrollX: number;
  cameraScrollY: number;
  transitionFlashActive: boolean;
  npcGlowTarget: string | null;
}
