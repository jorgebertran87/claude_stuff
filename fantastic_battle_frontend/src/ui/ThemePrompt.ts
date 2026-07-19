export class ThemePrompt {
  private container: HTMLDivElement | null = null;

  show(): Promise<string> {
    this.remove();

    this.container = document.createElement("div");
    this.container.id = "theme-prompt";
    this.container.style.cssText =
      "position:fixed;top:0;left:0;width:100%;height:100%;" +
      "background:rgba(0,0,0,0.85);display:flex;flex-direction:column;" +
      "align-items:center;justify-content:center;z-index:1000;" +
      "color:#fff;font-family:sans-serif;";

    const title = document.createElement("h1");
    title.textContent = "Fantastic Battle";
    title.style.cssText = "font-size:36px;margin-bottom:10px;";

    const subtitle = document.createElement("p");
    subtitle.textContent = "Choose a theme for your adventure";
    subtitle.style.cssText = "font-size:18px;margin-bottom:24px;color:#aaa;";

    const input = document.createElement("input");
    input.id = "theme-input";
    input.type = "text";
    input.placeholder = "e.g. Greek mythology, Computer Science...";
    input.style.cssText =
      "padding:10px 16px;font-size:18px;border-radius:4px;border:none;" +
      "width:350px;margin-bottom:16px;text-align:center;";

    const button = document.createElement("button");
    button.id = "theme-start";
    button.textContent = "Start Game";
    button.style.cssText =
      "padding:10px 32px;font-size:18px;border-radius:4px;border:none;" +
      "cursor:pointer;background:#4caf50;color:#fff;";

    this.container.appendChild(title);
    this.container.appendChild(subtitle);
    this.container.appendChild(input);
    this.container.appendChild(button);

    document.body.appendChild(this.container);

    return new Promise((resolve) => {
      const submit = () => {
        const value = input.value.trim();
        if (!value) {
          input.style.border = "2px solid #f44336";
          return;
        }
        button.removeEventListener("click", submit);
        input.removeEventListener("keydown", onKey);
        resolve(value);
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

  remove(): void {
    if (this.container) {
      this.container.remove();
      this.container = null;
    }
  }
}
