interface Position {
  x: number;
  y: number;
}

interface NpcData {
  name: string;
  position: Position;
  direction: string;
}

export interface SessionResponse {
  id: string;
  player_position: Position;
  player_direction: string;
  npcs: NpcData[];
  map: {
    width: number;
    height: number;
    tiles: { x: number; y: number; type: string }[];
    start_position: Position;
  };
}

interface MoveResponse {
  player_position: Position;
  player_direction: string;
}

export interface InteractResponse {
  npc: NpcData | null;
  battle: { question: string } | null;
}

export class ApiClient {
  private baseUrl: string;
  private sessionId: string | null = null;

  constructor(baseUrl: string = "http://localhost:8080") {
    this.baseUrl = baseUrl;
  }

  async join(theme?: string): Promise<SessionResponse> {
    const body = theme ? JSON.stringify({ theme }) : undefined;
    const resp = await fetch(`${this.baseUrl}/api/sessions`, {
      method: "POST",
      headers: body ? { "Content-Type": "application/json" } : undefined,
      body,
    });
    if (!resp.ok) {
      throw new Error(`join failed: ${resp.status}`);
    }
    const data = await resp.json();
    this.sessionId = data.id;
    return data;
  }

  async move(direction: string): Promise<MoveResponse> {
    if (!this.sessionId) {
      throw new Error("no session");
    }
    const capitalized =
      direction.charAt(0).toUpperCase() + direction.slice(1).toLowerCase();
    const resp = await fetch(
      `${this.baseUrl}/api/sessions/${this.sessionId}/move`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ direction: capitalized }),
      }
    );
    if (!resp.ok) {
      throw new Error(`move failed: ${resp.status}`);
    }
    return resp.json();
  }

  async interact(): Promise<InteractResponse> {
    if (!this.sessionId) {
      throw new Error("no session");
    }
    const resp = await fetch(
      `${this.baseUrl}/api/sessions/${this.sessionId}/interact`,
      { method: "POST" }
    );
    if (!resp.ok) {
      throw new Error(`interact failed: ${resp.status}`);
    }
    return resp.json();
  }

  async answer(answer: string): Promise<{ outcome: string }> {
    if (!this.sessionId) {
      throw new Error("no session");
    }
    const resp = await fetch(
      `${this.baseUrl}/api/sessions/${this.sessionId}/battle/answer`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ answer }),
      }
    );
    if (!resp.ok) {
      throw new Error(`answer failed: ${resp.status}`);
    }
    return resp.json();
  }

  getSessionId(): string | null {
    return this.sessionId;
  }
}
