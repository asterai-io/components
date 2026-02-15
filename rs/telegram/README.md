# asterai:telegram

Telegram integration for asterai.
Send messages via the Bot API and listen for
incoming messages via webhooks.

## Interface

```wit
package asterai:telegram@0.1.0;

interface api {
  get-self: func() -> user;
  send-message: func(
    content: string,
    chat-id: s64,
  ) -> string;
}

interface incoming-handler {
  use asterai:telegram/types@0.1.0.{message};
  on-message: func(message: message);
}
```

`send-message` returns the message ID on success,
or `"error: ..."` on failure.

## Environment Variables

| Variable                               | Required | Description                                      |
|----------------------------------------|----------|--------------------------------------------------|
| `TELEGRAM_TOKEN`                       | Yes      | Bot token from BotFather                         |
| `TELEGRAM_WEBHOOK_URL`                 | Yes      | Public URL for receiving webhook updates         |
| `TELEGRAM_WEBHOOK_SECRET`              | Yes      | Secret token for validating webhook requests     |
| `TELEGRAM_INCOMING_HANDLER_COMPONENTS` | No       | Comma-separated list of consumer component names |

### Setup

1. **Create a bot** — open Telegram and message
   [@BotFather](https://t.me/BotFather).
   Send `/newbot`, follow the prompts, and copy the
   token it gives you.
   Set it as `TELEGRAM_TOKEN`.

2. **Set up a webhook URL ENV var** — Telegram needs a public
   HTTPS URL to send updates to. This is the URL where
   your asterai environment receives HTTP requests.
   Set it as `TELEGRAM_WEBHOOK_URL`
   (e.g. `https://your-domain.com/your-username/your-env-name/asterai/telegram/webhook`).
   Note this step only requires setting the ENV var,
   no setup is needed on Telegram's side other than
   getting the TELEGRAM_TOKEN.
   The component automatically calls Telegram's
   `setWebhook` API on startup to register the URL
   and secret.

3. **Choose a webhook secret** — pick any random string
   (e.g. `openssl rand -hex 32`). Telegram will include
   this in the `X-Telegram-Bot-Api-Secret-Token` header
   on every webhook request so the component can verify
   it. Set it as `TELEGRAM_WEBHOOK_SECRET`.

4. **Configure handler components** (optional) — if you
   want to listen for incoming messages, set
   `TELEGRAM_INCOMING_HANDLER_COMPONENTS` to a
   comma-separated list of components that implement
   `asterai:telegram/incoming-handler@0.1.0`.

## Usage

### Sending messages

```bash
asterai env add-component my-env asterai:telegram
asterai env set-var my-env \
  TELEGRAM_TOKEN="your-bot-token"
asterai env set-var my-env \
  TELEGRAM_WEBHOOK_URL="https://your-domain.com/webhook"
asterai env set-var my-env \
  TELEGRAM_WEBHOOK_SECRET="your-secret"

asterai env call my-env asterai:telegram \
  api/send-message "Hello from asterai!" 123456789
```

### Listening for messages

On startup, the component registers the webhook with
Telegram via `setWebhook`. When a message arrives,
it fans out to all components listed in
`TELEGRAM_INCOMING_HANDLER_COMPONENTS`.

```bash
asterai env set-var my-env \
  TELEGRAM_INCOMING_HANDLER_COMPONENTS=\
"my-username:my-bot,my-username:my-logger"
```

Each target must export the `incoming-handler`
interface:

```wit
package my-username:my-bot@0.1.0;

world component {
  import asterai:host/api@1.0.0;
  export asterai:telegram/incoming-handler@0.1.0;
}
```

#### Message flow

```
Telegram Bot API
  │
  │  POST webhook (JSON update)
  │
  ▼
┌──────────────────────────────────┐
│ asterai:telegram                 │
│                                  │
│  wasi:http/incoming-handler      │
│    ├─► my-username:my-bot        │
│    └─► my-username:my-logger     │
└──────────────────────────────────┘
```
