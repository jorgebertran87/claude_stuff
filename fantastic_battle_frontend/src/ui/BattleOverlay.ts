export class BattleOverlay {
  private container: HTMLDivElement | null = null;

  show(question: string): Promise<string> {
    this.remove();

    this.container = document.createElement("div");
    this.container.id = "battle-overlay";
    this.container.style.cssText =
      "position:fixed;top:0;left:0;width:100%;height:100%;" +
      "background:rgba(0,0,0,0.85);display:flex;flex-direction:column;" +
      "align-items:center;justify-content:center;z-index:1000;" +
      "color:#fff;font-family:sans-serif;";

    const questionEl = document.createElement("p");
    questionEl.id = "battle-question";
    questionEl.textContent = question;
    questionEl.style.cssText = "font-size:24px;margin-bottom:20px;text-align:center;";

    const input = document.createElement("input");
    input.id = "battle-answer-input";
    input.type = "text";
    input.placeholder = "Your answer...";
    input.style.cssText =
      "padding:8px 12px;font-size:18px;border-radius:4px;border:none;" +
      "width:300px;margin-bottom:12px;";

    const button = document.createElement("button");
    button.id = "battle-submit";
    button.textContent = "Answer";
    button.style.cssText =
      "padding:8px 24px;font-size:16px;border-radius:4px;border:none;" +
      "cursor:pointer;background:#4caf50;color:#fff;";

    this.container.appendChild(questionEl);
    this.container.appendChild(input);
    this.container.appendChild(button);

    document.body.appendChild(this.container);

    return new Promise((resolve) => {
      const submit = () => {
        button.removeEventListener("click", submit);
        input.removeEventListener("keydown", onKey);
        resolve(input.value);
      };
      const onKey = (e: KeyboardEvent) => {
        if (e.key === "Enter") {
          submit();
        }
      };
      button.addEventListener("click", submit);
      input.addEventListener("keydown", onKey);
      input.focus();
    });
  }

  showOutcome(outcome: string): Promise<void> {
    if (!this.container) {
      return Promise.resolve();
    }

    this.container.innerHTML = "";

    const outcomeEl = document.createElement("p");
    outcomeEl.id = "battle-outcome";
    outcomeEl.textContent = outcome;
    outcomeEl.style.cssText =
      "font-size:32px;font-weight:bold;text-align:center;";

    this.container.appendChild(outcomeEl);

    return new Promise((resolve) => {
      setTimeout(() => {
        this.remove();
        resolve();
      }, 2000);
    });
  }

  remove(): void {
    if (this.container) {
      this.container.remove();
      this.container = null;
    }
  }
}
