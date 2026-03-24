from datetime import datetime
from dotenv import load_dotenv
import asyncio
import os
from pathlib import Path

# Load environment variables from the .env file
load_dotenv()

from claude_agent_sdk import ClaudeAgentOptions, ResultMessage, SystemMessage, query

from voice_listener.domain.ports import OrderHandler

_PROMPT_FILE = Path(__file__).parent / "prompt"
_TOKENS_LOG_FILE = Path(__file__).parents[2] / ".orders_tokens"

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

    def handle(self, order: str) -> str:
        return asyncio.run(self._dispatch(order))

    async def _dispatch(self, order: str) -> str:
        options = ClaudeAgentOptions(
            model=self._model,
            system_prompt=_PROMPT,
            allowed_tools=["Bash", "WebSearch"],
        )

        result = "Sin respuesta."
        try:
            async for message in query(prompt=order, options=options):
                if isinstance(message, SystemMessage) and message.subtype == "init":
                    print(f"Claude Code session: {message.data.get('session_id')}")
                if isinstance(message, ResultMessage):
                    result = message.result
                    usage = message.usage or {}
                    input_tokens = usage.get("input_tokens", 0)
                    output_tokens = usage.get("output_tokens", 0)
                    cache_read = usage.get("cache_read_input_tokens", 0)
                    cache_creation = usage.get("cache_creation_input_tokens", 0)
                    total_tokens = input_tokens + output_tokens + cache_read + cache_creation
                    log_line = (
                        f"{datetime.now().isoformat()} | Claude order: {order} | "
                        f"Tokens used — input: {input_tokens}, output: {output_tokens}, "
                        f"cache_read: {cache_read}, cache_creation: {cache_creation}, "
                        f"total: {total_tokens} | cost: ${message.total_cost_usd:.6f} USD"
                    )
                    with _TOKENS_LOG_FILE.open("a", encoding="utf-8") as f:
                        print(log_line, file=f)
        except Exception as e:
            return "No tienes tokens disponibles. Por favor, revisa tu configuración."

        return result
