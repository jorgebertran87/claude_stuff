from dotenv import load_dotenv
import asyncio
import os

# Load environment variables from the .env file
load_dotenv()

# Access environment variables
default_user_city = os.getenv("DEFAULT_USER_CITY")
voice_language = os.getenv("VOICE_LANGUAGE")
wake_word = os.getenv("WAKE_WORD")

from claude_agent_sdk import ClaudeAgentOptions, ResultMessage, SystemMessage, query

from voice_listener.domain.ports import OrderHandler

_SYSTEM_PROMPT = (
    "Responde de forma concisa y directa a las órdenes del usuario."
    "Si te pide que hagas algo, hazlo sin preguntar. Si no puedes acceder a la información, utiliza la tool Web Search para buscarla y luego responde."
    "No le preguntes si necesita algo más, solo haz lo que te pida."
    "Si te pregunta que tiempo hace, si no te indica ninguna ciudad o ubicación,  responde con el clima actual de " + default_user_city + ". Cuando me contestes, indica qué ciudad has usado para responder. "
    "Devuelve la información en texto plano, sin formato ni explicaciones adicionales. Solamente la información relevante, nada más. Si no te pido que extiendas la respuesta, hazla lo más resumida posible"
    "Devuelve la respuesta en el idioma oficial del país con código '" + voice_language + "'."
)


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
            system_prompt=_SYSTEM_PROMPT,
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
