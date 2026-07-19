export class ResultsOverlay {
  private container: HTMLDivElement | null = null;

  show(results: {
    total: number;
    correct: number;
    incorrect: number;
  }): Promise<void> {
    this.remove();

    this.container = document.createElement("div");
    this.container.id = "results-overlay";
    this.container.style.cssText =
      "position:fixed;top:0;left:0;width:100%;height:100%;" +
      "background:rgba(0,0,0,0.85);display:flex;flex-direction:column;" +
      "align-items:center;justify-content:center;z-index:1000;" +
      "color:#fff;font-family:sans-serif;";

    const title = document.createElement("h1");
    title.textContent = "Adventure Complete!";
    title.style.cssText = "font-size:36px;margin-bottom:24px;";

    const correctLine = document.createElement("p");
    correctLine.textContent = `Correct: ${results.correct}`;
    correctLine.style.cssText =
      "font-size:24px;margin:8px 0;color:#4caf50;";

    const incorrectLine = document.createElement("p");
    incorrectLine.textContent = `Incorrect: ${results.incorrect}`;
    incorrectLine.style.cssText =
      "font-size:24px;margin:8px 0;color:#f44336;";

    const totalLine = document.createElement("p");
    totalLine.textContent = `Total: ${results.total}`;
    totalLine.style.cssText =
      "font-size:24px;margin:8px 0 32px 0;color:#fff;";

    const button = document.createElement("button");
    button.id = "results-play-again";
    button.textContent = "Play Again";
    button.style.cssText =
      "padding:12px 40px;font-size:20px;border-radius:4px;border:none;" +
      "cursor:pointer;background:#4caf50;color:#fff;";

    this.container.appendChild(title);
    this.container.appendChild(correctLine);
    this.container.appendChild(incorrectLine);
    this.container.appendChild(totalLine);
    this.container.appendChild(button);

    document.body.appendChild(this.container);

    return new Promise((resolve) => {
      button.addEventListener("click", () => {
        resolve();
      });
    });
  }

  remove(): void {
    if (this.container) {
      this.container.remove();
      this.container = null;
    }
  }
}
