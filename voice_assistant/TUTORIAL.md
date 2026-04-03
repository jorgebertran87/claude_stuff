# Claudito — Beginner Tutorial

This guide takes you from zero to a working assistant, step by step.

---

## What is Claudito?

Claudito is a personal voice assistant that:
- Listens for the wake word **"Claudito"** through the microphone
- Sends your question to Claude (Anthropic's AI)
- Speaks the response aloud

It also works as a **Telegram bot**: write to it from your phone and it replies like a smart chat assistant.

---

## What you need before starting

| Requirement | What for |
|---|---|
| Linux computer (or Raspberry Pi) | Where the assistant will run |
| [Docker](https://docs.docker.com/engine/install/) | To run the assistant without installing dependencies |
| [Anthropic](https://console.anthropic.com) account | To use Claude AI |
| [Claude Code CLI](https://claude.ai/code) installed | The bridge between Claudito and the AI |
| (Optional) Telegram account and a bot created | To use it from your phone |

---

## Step 1 — Install Docker

If you don't have Docker installed:

```bash
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
# Log out and back in for the group change to take effect
```

Verify it works:

```bash
docker run hello-world
```

---

## Step 2 — Install Claude Code CLI and authenticate

```bash
npm install -g @anthropic-ai/claude-code
claude login
```

`claude login` will open the browser so you can sign in with your Anthropic account. This only needs to be done once.

---

## Step 3 — Download the project

```bash
git clone https://github.com/<username>/voice_assistant.git
cd voice_assistant/rust
```

---

## Step 4 — Create the configuration file

Copy the example file and fill it in:

```bash
cp .env.example .env
```

Open `.env` with any text editor and fill in the fields:

```env
DEFAULT_USER_CITY=Madrid          # Your city, for weather queries
VOICE_LANGUAGE=es-ES              # Language (es-ES, en-US, etc.)
WAKE_WORD=Claudito                # Wake word
TELEGRAM_BOT_TOKEN=               # Only if using Telegram (see Step 6)
TELEGRAM_ALLOWED_CHAT_IDS=        # Leave empty to allow any chat
DOCKER_USERNAME=                  # Your Docker Hub username (only for publishing)
BT_SPEAKER_MAC=                   # MAC address of your Bluetooth speaker (optional)
CUENTAS_SHEET_NAME=               # Name of your spreadsheet (optional)
```

Save the file.

---

## Step 5 — Build the Docker image

From the `rust/` folder:

```bash
make build
```

This may take several minutes the first time. It only needs to be done again when the code changes.

---

## Step 6 — Voice mode (microphone + speaker)

Make sure your system uses PipeWire or PulseAudio (most modern Linux distributions do).

```bash
make run
```

You will see in the terminal:

```
Waiting for wake word "claudito"...
```

Say **"Claudito"** out loud. The assistant will respond.

To stop: press `Ctrl+C`.

---

## Step 7 — Telegram mode (optional)

### 7.1 Create a Telegram bot

1. Open Telegram and search for **@BotFather**
2. Type `/newbot` and follow the instructions
3. At the end it will give you a token that looks like: `123456789:ABCdef...`
4. Copy it into `.env` as `TELEGRAM_BOT_TOKEN=123456789:ABCdef...`

### 7.2 Get your chat ID (optional, to restrict access)

1. Search for **@userinfobot** in Telegram and send it any message
2. It will tell you your numeric ID (for example `987654321`)
3. Put it in `.env` as `TELEGRAM_ALLOWED_CHAT_IDS=987654321`

If left empty, anyone who finds your bot will be able to use it.

### 7.3 Start the bot

```bash
make run-telegram
```

Open Telegram, find your bot by the name you gave it, and write to it. It will reply using Claude.

---

## Available Telegram commands

Type them directly in the chat:

| Command | What it does |
|---|---|
| `/list` | Show all available commands |
| `/reset` | Clear the conversation and start fresh |
| `/usage` | Show how many tokens and money you have spent |
| `/voice_mode` | Toggle spoken responses through the computer speaker |
| `/volume [+N\|-N\|N]` | Raise, lower or check the volume (e.g. `/volume 70`, `/volume +10`) |
| `/cuentas` | Analyse your Google Sheets spreadsheet (requires setup) |
| `/auth_google` | Connect your Google account to use `/cuentas` |

---

## Step 8 — Connect Google Sheets for `/cuentas` (optional)

This allows the bot to analyse your personal spreadsheet.

### 8.1 Create credentials in Google Cloud

1. Go to [Google Cloud Console](https://console.cloud.google.com)
2. Create a new project (or use an existing one)
3. Enable the **Google Sheets API**: APIs & Services → Library → search "Google Sheets API" → Enable
4. Go to APIs & Services → Credentials → Create Credentials → **OAuth 2.0 Client ID**
5. Application type: **Desktop app**
6. Download or copy the **Client ID** and **Client Secret**

### 8.2 Configure the consent screen

1. Go to APIs & Services → OAuth consent screen
2. Type: **External**
3. Fill in the app name and email
4. Under **Scopes**, add: `https://www.googleapis.com/auth/spreadsheets.readonly`
5. Under **Test users**, add your Google email
6. Save

### 8.3 Add the credentials to `.env`

```env
CUENTAS_SHEET_NAME=My Spreadsheet       # Exact name of your sheet
GOOGLE_SPREADSHEET_ID=                  # ID from the sheet URL (see below)
GOOGLE_CLIENT_ID=                       # Client ID from the previous step
GOOGLE_CLIENT_SECRET=                   # Client Secret from the previous step
GOOGLE_REFRESH_TOKEN=                   # Generated in step 8.4
```

The **Spreadsheet ID** is in the URL of your sheet:
`https://docs.google.com/spreadsheets/d/`**`THIS_IS_THE_ID`**`/edit`

### 8.4 Authorise access from Telegram

With the bot running, write to it:

```
/auth_google
```

The bot will send you a link. Open it in the browser, accept the permissions, and copy the code that appears. Send it back to the bot. Done — the token is saved and you can now use `/cuentas`.

---

## Common troubleshooting

**"Microphone not heard"**
Check that PulseAudio or PipeWire is running: `pactl info`

**"Wake word not detected"**
Speak closer to the microphone or try a lower threshold. You can also change the word in `.env`.

**"No tienes tokens disponibles" (No tokens available)**
Your Anthropic account has no credit. Top it up at [console.anthropic.com](https://console.anthropic.com).

**"Error al acceder a Google Sheets" (Error accessing Google Sheets)**
Run `/auth_google` again to renew the token.

**Telegram bot not responding**
Verify that `TELEGRAM_BOT_TOKEN` is correct in `.env` and that the container is running (`docker ps`).

---

## Updating to the latest version

```bash
git pull
make build
make run-telegram   # or make run
```

---

---

# Claudito — Tutorial para principiantes

Esta guía te lleva desde cero hasta tener el asistente funcionando, paso a paso.

---

## ¿Qué es Claudito?

Claudito es un asistente de voz personal que:
- Escucha la palabra de activación **"Claudito"** por el micrófono
- Envía tu pregunta a Claude (la IA de Anthropic)
- Te responde en voz alta

También funciona como **bot de Telegram**: le escribes desde el móvil y te responde como un chat inteligente.

---

## Lo que necesitas antes de empezar

| Requisito | Para qué |
|---|---|
| Ordenador con Linux (o Raspberry Pi) | Donde correrá el asistente |
| [Docker](https://docs.docker.com/engine/install/) | Para ejecutar el asistente sin instalar dependencias |
| Cuenta en [Anthropic](https://console.anthropic.com) | Para usar la IA de Claude |
| [Claude Code CLI](https://claude.ai/code) instalado | El puente entre Claudito y la IA |
| (Opcional) Cuenta de Telegram y un bot creado | Para usarlo desde el móvil |

---

## Paso 1 — Instala Docker

Si no tienes Docker instalado:

```bash
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
# Cierra sesión y vuelve a entrar para que el grupo surta efecto
```

Verifica que funciona:

```bash
docker run hello-world
```

---

## Paso 2 — Instala Claude Code CLI y autentícate

```bash
npm install -g @anthropic-ai/claude-code
claude login
```

`claude login` abrirá el navegador para que inicies sesión con tu cuenta de Anthropic. Sólo hay que hacerlo una vez.

---

## Paso 3 — Descarga el proyecto

```bash
git clone https://github.com/<usuario>/voice_assistant.git
cd voice_assistant/rust
```

---

## Paso 4 — Crea el fichero de configuración

Copia el fichero de ejemplo y rellénalo:

```bash
cp .env.example .env
```

Abre `.env` con cualquier editor de texto y rellena los campos:

```env
DEFAULT_USER_CITY=Madrid          # Tu ciudad, para consultas del tiempo
VOICE_LANGUAGE=es-ES              # Idioma (es-ES, en-US, etc.)
WAKE_WORD=Claudito                # Palabra de activación
TELEGRAM_BOT_TOKEN=               # Sólo si usas Telegram (ver Paso 6)
TELEGRAM_ALLOWED_CHAT_IDS=        # Deja vacío para permitir cualquier chat
DOCKER_USERNAME=                  # Tu usuario de Docker Hub (sólo para publicar)
BT_SPEAKER_MAC=                   # MAC de tu altavoz Bluetooth (opcional)
CUENTAS_SHEET_NAME=               # Nombre de tu hoja de cálculo (opcional)
```

Guarda el fichero.

---

## Paso 5 — Construye la imagen Docker

Desde la carpeta `rust/`:

```bash
make build
```

Esto puede tardar varios minutos la primera vez. Sólo es necesario hacerlo cuando el código cambia.

---

## Paso 6 — Modo voz (micrófono + altavoz)

Asegúrate de que tu sistema usa PipeWire o PulseAudio (la mayoría de distribuciones Linux modernas lo hacen).

```bash
make run
```

Verás en la terminal:

```
Waiting for wake word "claudito"...
```

Di **"Claudito"** en voz alta. El asistente responderá.

Para parar: pulsa `Ctrl+C`.

---

## Paso 7 — Modo Telegram (opcional)

### 7.1 Crea un bot de Telegram

1. Abre Telegram y busca **@BotFather**
2. Escribe `/newbot` y sigue las instrucciones
3. Al final te dará un token con este aspecto: `123456789:ABCdef...`
4. Cópialo en `.env` como `TELEGRAM_BOT_TOKEN=123456789:ABCdef...`

### 7.2 Obtén tu chat ID (opcional, para restringir acceso)

1. Busca **@userinfobot** en Telegram y escríbele cualquier cosa
2. Te dirá tu ID numérico (por ejemplo `987654321`)
3. Ponlo en `.env` como `TELEGRAM_ALLOWED_CHAT_IDS=987654321`

Si lo dejas vacío, cualquiera que encuentre tu bot podrá usarlo.

### 7.3 Arranca el bot

```bash
make run-telegram
```

Abre Telegram, busca tu bot por el nombre que le pusiste, y escríbele. Responderá con Claude.

---

## Comandos disponibles en Telegram

Escríbelos directamente en el chat:

| Comando | Qué hace |
|---|---|
| `/list` | Muestra todos los comandos disponibles |
| `/reset` | Borra la conversación y empieza de cero |
| `/usage` | Muestra cuántos tokens y dinero llevas gastado |
| `/voice_mode` | Activa/desactiva que el bot hable por el altavoz del ordenador |
| `/volume [+N\|-N\|N]` | Sube, baja o consulta el volumen (ej: `/volume 70`, `/volume +10`) |
| `/cuentas` | Analiza tu hoja de cálculo de Google Sheets (requiere configuración) |
| `/auth_google` | Conecta tu cuenta de Google para usar `/cuentas` |

---

## Paso 8 — Conectar Google Sheets para `/cuentas` (opcional)

Esto permite que el bot analice tu hoja de cálculo personal.

### 8.1 Crea credenciales en Google Cloud

1. Ve a [Google Cloud Console](https://console.cloud.google.com)
2. Crea un proyecto nuevo (o usa uno existente)
3. Activa la **Google Sheets API**: APIs y servicios → Biblioteca → busca "Google Sheets API" → Activar
4. Ve a APIs y servicios → Credenciales → Crear credenciales → **ID de cliente OAuth 2.0**
5. Tipo de aplicación: **Aplicación de escritorio**
6. Descarga o copia el **Client ID** y el **Client Secret**

### 8.2 Configura la pantalla de consentimiento

1. Ve a APIs y servicios → Pantalla de consentimiento de OAuth
2. Tipo: **Externo**
3. Rellena nombre de la app y correo
4. En **Ámbitos**, añade: `https://www.googleapis.com/auth/spreadsheets.readonly`
5. En **Usuarios de prueba**, añade tu correo de Google
6. Guarda

### 8.3 Añade las credenciales al `.env`

```env
CUENTAS_SHEET_NAME=Cuentas Personales   # Nombre exacto de tu hoja
GOOGLE_SPREADSHEET_ID=                  # ID de la URL de tu hoja (ver abajo)
GOOGLE_CLIENT_ID=                       # Client ID del paso anterior
GOOGLE_CLIENT_SECRET=                   # Client Secret del paso anterior
GOOGLE_REFRESH_TOKEN=                   # Se genera en el paso 8.4
```

El **Spreadsheet ID** está en la URL de tu hoja:
`https://docs.google.com/spreadsheets/d/`**`ESTE_ES_EL_ID`**`/edit`

### 8.4 Autoriza el acceso desde Telegram

Con el bot corriendo, escríbele:

```
/auth_google
```

El bot te enviará un enlace. Ábrelo en el navegador, acepta los permisos, y copia el código que aparece. Envíaselo al bot. Listo — el token queda guardado y ya puedes usar `/cuentas`.

---

## Solución de problemas frecuentes

**"No se escucha el micrófono"**
Comprueba que PulseAudio o PipeWire está corriendo: `pactl info`

**"Wake word not detected"**
Habla más cerca del micrófono o prueba con un umbral más bajo. También puedes cambiar la palabra en `.env`.

**"No tienes tokens disponibles"**
Tu cuenta de Anthropic no tiene crédito. Recárgala en [console.anthropic.com](https://console.anthropic.com).

**"Error al acceder a Google Sheets"**
Ejecuta `/auth_google` de nuevo para renovar el token.

**El bot de Telegram no responde**
Verifica que `TELEGRAM_BOT_TOKEN` es correcto en `.env` y que el contenedor está corriendo (`docker ps`).

---

## Actualizar a la última versión

```bash
git pull
make build
make run-telegram   # o make run
```
