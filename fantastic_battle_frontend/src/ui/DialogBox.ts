import Phaser from "phaser";

const TYPEWRITER_DELAY = 50;

export class DialogBox {
  private scene: Phaser.Scene;
  private background: Phaser.GameObjects.Rectangle;
  private textObj: Phaser.GameObjects.Text;
  private indicator: Phaser.GameObjects.Triangle;
  private fullText: string;
  private currentIndex: number;
  private typewriterTimer: Phaser.Time.TimerEvent | null;
  private indicatorTween: Phaser.Tweens.Tween | null;
  private resolvePromise: (() => void) | null;
  private typewriterComplete: boolean;
  private destroyed: boolean;
  private spacePressed: boolean;
  private keyDownHandler: (event: KeyboardEvent) => void;

  constructor(scene: Phaser.Scene, text: string) {
    this.scene = scene;
    this.fullText = text;
    this.currentIndex = 0;
    this.typewriterTimer = null;
    this.indicatorTween = null;
    this.resolvePromise = null;
    this.typewriterComplete = false;
    this.destroyed = false;
    this.spacePressed = false;

    this.background = scene.add.rectangle(400, 520, 760, 140, 0x000000, 0.85);
    this.background.setStrokeStyle(2, 0xffffff);
    this.background.setDepth(200);
    this.background.setScrollFactor(0);

    this.textObj = scene.add.text(50, 470, "", {
      fontSize: "16px",
      fontFamily: "monospace",
      color: "#ffffff",
      wordWrap: { width: 700 },
    });
    this.textObj.setDepth(201);
    this.textObj.setScrollFactor(0);

    this.indicator = scene.add.triangle(740, 565, 0, 0, 8, 6, 0, 12, 0xffffff);
    this.indicator.setDepth(201);
    this.indicator.setScrollFactor(0);
    this.indicator.setAlpha(0);

    this.keyDownHandler = (event: KeyboardEvent) => {
      if (event.code === "Space") {
        event.preventDefault();
        this.spacePressed = true;
      }
    };
    document.addEventListener("keydown", this.keyDownHandler);

    this.startTypewriter();
    this.exposeState();
  }

  show(): Promise<void> {
    return new Promise((resolve) => {
      this.resolvePromise = resolve;
    });
  }

  update(): void {
    if (this.destroyed) {
      return;
    }

    if (this.spacePressed) {
      this.spacePressed = false;
      if (!this.typewriterComplete) {
        this.finishTypewriter();
      } else {
        this.dismiss();
      }
    }
  }

  destroy(): void {
    if (this.destroyed) {
      return;
    }
    this.destroyed = true;
    document.removeEventListener("keydown", this.keyDownHandler);
    if (this.typewriterTimer) {
      this.typewriterTimer.destroy();
      this.typewriterTimer = null;
    }
    if (this.indicatorTween) {
      this.indicatorTween.destroy();
      this.indicatorTween = null;
    }
    this.background.destroy();
    this.textObj.destroy();
    this.indicator.destroy();
    (window as any).__dialogState = null;
  }

  private startTypewriter(): void {
    this.typewriterTimer = this.scene.time.addEvent({
      delay: TYPEWRITER_DELAY,
      callback: () => {
        this.currentIndex++;
        this.textObj.text = this.fullText.substring(0, this.currentIndex);
        this.exposeState();
        if (this.currentIndex >= this.fullText.length) {
          this.onTypewriterComplete();
        }
      },
      repeat: this.fullText.length - 1,
    });
  }

  private finishTypewriter(): void {
    if (this.typewriterTimer) {
      this.typewriterTimer.destroy();
      this.typewriterTimer = null;
    }
    this.currentIndex = this.fullText.length;
    this.textObj.text = this.fullText;
    this.onTypewriterComplete();
  }

  private onTypewriterComplete(): void {
    this.typewriterComplete = true;
    this.indicatorTween = this.scene.tweens.add({
      targets: this.indicator,
      alpha: 1,
      duration: 300,
      yoyo: true,
      repeat: -1,
    });
    this.exposeState();
  }

  private dismiss(): void {
    this.destroy();
    if (this.resolvePromise) {
      this.resolvePromise();
      this.resolvePromise = null;
    }
  }

  private exposeState(): void {
    (window as any).__dialogState = {
      visible: !this.destroyed,
      displayedLength: this.currentIndex,
      totalLength: this.fullText.length,
      typewriterComplete: this.typewriterComplete,
    };
  }
}
