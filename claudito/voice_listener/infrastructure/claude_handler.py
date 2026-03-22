from dotenv import load_dotenv
import asyncio
import os
from pathlib import Path

# Load environment variables from the .env file
load_dotenv()

from claude_agent_sdk import ClaudeAgentOptions, ResultMessage, SystemMessage, query

from voice_listener.domain.ports import OrderHandler

_PROMPT_FILE = Path(__file__).parent / "prompt"

def _load_prompt() -> str:
    template = "No se pudo cargar el prompt."
    try:
        template = _PROMPT_FILE.read_text(encoding="utf-8")
    except Exception:
        template = Path(__file__).parent / "prompt.example"
    return template.format(
        default_user_city=os.getenv("DEFAULT_USER_CITY", ""),
        voice_language=os.getenv("VOICE_LANGUAGE", "es"),
    )

_PROMPT = _load_prompt()


class ClaudeCodeHandler(OrderHandler):
    def __init__(self, model: str = "claude-haiku-4-5") -> None:
        self._model = model
        self._session_id: str | None = os.environ.get("CLAUDE_SESSION_ID")
        if self._session_id:
            print(f"Resuming Claude Code session: {self._session_id}")

    def handle(self, order: str) -> str:
        return asyncio.run(self._dispatch(order))

    async def _dispatch(self, order: str) -> str:
        options = ClaudeAgentOptions(
            model=self._model,
            system_prompt=_PROMPT,
            allowed_tools=["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch"],
            resume=self._session_id,
        )

        result = "Sin respuesta."
        async for message in query(prompt=order, options=options):
            if isinstance(message, SystemMessage) and message.subtype == "init":
                self._session_id = message.data.get("session_id")
                print(f"Claude Code session: {self._session_id}")
            if isinstance(message, ResultMessage):
                result = message.result

        return result
