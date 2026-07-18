export interface NpcState {
  name: string;
  position: { x: number; y: number };
  direction: string;
}

export interface GameState {
  playerPosition: { x: number; y: number };
  playerDirection: string;
  isMoving: boolean;
  mapWidth: number;
  mapHeight: number;
  npcs: NpcState[];
}
