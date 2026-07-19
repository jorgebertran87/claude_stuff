export class SoundService {
  private audioContext: AudioContext | null = null;
  private bgmPlaying = false;
  private bgmOscillators: OscillatorNode[] = [];
  private _lastEffectPlayed: string | null = null;

  private getContext(): AudioContext | null {
    if (!this.audioContext) {
      try {
        this.audioContext = new AudioContext();
      } catch {
        return null;
      }
    }
    if (this.audioContext.state === "suspended") {
      this.audioContext.resume();
    }
    return this.audioContext;
  }

  get lastEffectPlayed(): string | null {
    return this._lastEffectPlayed;
  }

  get isBgmPlaying(): boolean {
    return this.bgmPlaying;
  }

  playFootstep(): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const bufferSize = ctx.sampleRate * 0.05;
    const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
    const data = buffer.getChannelData(0);
    for (let i = 0; i < bufferSize; i++) {
      data[i] = (Math.random() * 2 - 1) * 0.1;
    }
    const source = ctx.createBufferSource();
    source.buffer = buffer;
    const filter = ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = 200;
    source.connect(filter);
    filter.connect(ctx.destination);
    source.start();
    this._lastEffectPlayed = "footstep";
    this.exposeState();
  }

  playBattleStart(): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const osc = ctx.createOscillator();
    osc.type = "square";
    osc.frequency.setValueAtTime(260, ctx.currentTime);
    osc.frequency.linearRampToValueAtTime(330, ctx.currentTime + 0.3);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.1, ctx.currentTime);
    gain.gain.linearRampToValueAtTime(0, ctx.currentTime + 0.35);
    osc.connect(gain);
    gain.connect(ctx.destination);
    osc.start();
    osc.stop(ctx.currentTime + 0.35);
    this._lastEffectPlayed = "battleStart";
    this.exposeState();
  }

  playVictory(): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const notes = [523, 659, 784, 1047];
    notes.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      osc.type = "square";
      osc.frequency.value = freq;
      const gain = ctx.createGain();
      gain.gain.setValueAtTime(0, ctx.currentTime + i * 0.15);
      gain.gain.linearRampToValueAtTime(0.1, ctx.currentTime + i * 0.15 + 0.01);
      gain.gain.linearRampToValueAtTime(0, ctx.currentTime + i * 0.15 + 0.15);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(ctx.currentTime + i * 0.15);
      osc.stop(ctx.currentTime + i * 0.15 + 0.15);
    });
    this._lastEffectPlayed = "victory";
    this.exposeState();
  }

  playDefeat(): void {
    const ctx = this.getContext();
    if (!ctx) return;
    const notes = [330, 262];
    notes.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      osc.type = "square";
      osc.frequency.value = freq;
      const gain = ctx.createGain();
      gain.gain.setValueAtTime(0, ctx.currentTime + i * 0.2);
      gain.gain.linearRampToValueAtTime(0.1, ctx.currentTime + i * 0.2 + 0.01);
      gain.gain.linearRampToValueAtTime(0, ctx.currentTime + i * 0.2 + 0.2);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(ctx.currentTime + i * 0.2);
      osc.stop(ctx.currentTime + i * 0.2 + 0.2);
    });
    this._lastEffectPlayed = "defeat";
    this.exposeState();
  }

  startBgm(): void {
    if (this.bgmPlaying) return;
    const ctx = this.getContext();
    if (!ctx) return;
    const now = ctx.currentTime;
    const melody = [262, 330, 392, 523, 392, 330, 262, 330];

    melody.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      osc.type = "square";
      osc.frequency.value = freq;
      const gain = ctx.createGain();
      const startTime = now + i * 0.4;
      gain.gain.setValueAtTime(0, startTime);
      gain.gain.linearRampToValueAtTime(0.05, startTime + 0.01);
      gain.gain.linearRampToValueAtTime(0, startTime + 0.35);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(startTime);
      osc.stop(startTime + 0.35);
      this.bgmOscillators.push(osc);
    });

    this.bgmPlaying = true;
    this.scheduleBgpRepeat(ctx, now + melody.length * 0.4, melody);
    this.exposeState();
  }

  private scheduleBgpRepeat(ctx: AudioContext, startAt: number, melody: number[]): void {
    const scheduleNext = () => {
      if (!this.bgmPlaying || !this.audioContext) return;
      const now = ctx.currentTime;
      melody.forEach((freq, i) => {
        const osc = ctx.createOscillator();
        osc.type = "square";
        osc.frequency.value = freq;
        const gain = ctx.createGain();
        const st = now + i * 0.4;
        gain.gain.setValueAtTime(0, st);
        gain.gain.linearRampToValueAtTime(0.05, st + 0.01);
        gain.gain.linearRampToValueAtTime(0, st + 0.35);
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.start(st);
        osc.stop(st + 0.35);
        this.bgmOscillators.push(osc);
      });
      const duration = melody.length * 0.4;
      setTimeout(scheduleNext, (duration - 0.1) * 1000);
    };
    const firstDelay = (startAt - ctx.currentTime) * 1000;
    setTimeout(scheduleNext, Math.max(0, firstDelay));
  }

  stopBgm(): void {
    this.bgmPlaying = false;
    for (const osc of this.bgmOscillators) {
      try { osc.stop(); } catch { /* already stopped */ }
    }
    this.bgmOscillators = [];
    this.exposeState();
  }

  private exposeState(): void {
    (window as any).__soundState = {
      lastEffectPlayed: this._lastEffectPlayed,
      bgmPlaying: this.bgmPlaying,
    };
  }
}
