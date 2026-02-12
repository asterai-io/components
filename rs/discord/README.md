# asterai:discord

Discord integration for asterai. Send messages via the REST API and listen for incoming messages via the Gateway WebSocket.

## Interface

```wit
package asterai:discord@0.1.0;

interface api {
  send-message: func(content: string, channel-id: string) -> string;
}

interface incoming-handler {
  use asterai:discord/types@0.1.0.{message};
  on-message: func(message: message);
}
```

`send-message` returns the message ID on success, or `"error: ..."` on failure.

## Environment Variables

| Variable                              | Required | Description                                      |
|---------------------------------------|----------|--------------------------------------------------|
| `DISCORD_TOKEN`                       | Yes      | Discord bot token                                |
| `DISCORD_INCOMING_HANDLER_COMPONENTS` | No       | Comma-separated list of consumer component names |

## Usage

### Sending messages

```bash
asterai env add-component my-env asterai:discord
asterai env set-var my-env DISCORD_TOKEN="your-bot-token"

asterai env call my-env asterai:discord api/send-message \
  "Hello from asterai!" "1234567890"
```

### Listening for messages

The component connects to the Discord Gateway WebSocket on startup via `asterai:host-ws`. When a message arrives, it fans out to all components listed in `DISCORD_INCOMING_HANDLER_COMPONENTS`.

```bash
asterai env set-var my-env DISCORD_INCOMING_HANDLER_COMPONENTS="my-username:my-bot,my-username:my-logger"
```

Each target must export the `incoming-handler` interface:

```wit
package my-username:my-bot@0.1.0;

world component {
  import asterai:host/api@1.0.0;
  export asterai:discord/incoming-handler@0.1.0;
}
```

#### Message flow

```
Discord Gateway (WebSocket)
  │
  │  asterai:host-ws manages the connection
  │
  ▼
┌────────────────────────────┐
│ asterai:discord             │
│                             │
│  on-message (from host-ws)  │
│    ├─► call-component-function
│    │   my-username:my-bot   │
│    └─► call-component-function
│        my-username:my-logger│
└────────────────────────────┘
```
