# asterai:discord-message-listener

A WASM component that listens for Discord messages via the Discord Gateway WebSocket and dispatches them to consumer components.

## How it works

WASI has no native WebSocket support (yet!), so this component uses `asterai:host-ws` to manage a persistent WebSocket connection to the Discord Gateway. The asterai runtime handles the actual socket lifecycle outside the WASM sandbox.

On startup (`wasi:cli/run`):

1. Reads the `DISCORD_LISTENER_TARGETS` environment variable
2. Calls `asterai:host-ws/connection::connect` to open a WebSocket to the Discord Gateway (with `auto_reconnect: true`)

When a Gateway message arrives, the runtime invokes the component's exported `asterai:host-ws/incoming-handler::on-message` handler. The component parses the Discord event and fans it out to all configured consumer components via `asterai:host/api::call-component-function`.

## Dynamic dispatch with `DISCORD_LISTENER_TARGETS`

Consumer components are configured at deploy time through the `DISCORD_LISTENER_TARGETS` environment variable. It takes a comma-separated list of component names:

```
DISCORD_LISTENER_TARGETS="my-username:my-bot,my-username:my-logger"
```

Each target must export the `incoming-handler` interface:

```wit
interface incoming-handler {
  use asterai:discord/types@0.1.0.{message};

  on-message: func(message: message);
}
```

When a Discord message is received, the component calls `on-message` on every target in the list. This lets you wire up multiple independent consumers (a bot, a logger, an analytics pipeline, etc.) to the same Discord event stream without any of them needing to manage their own Gateway connection.

### Consumer component example

A consumer just needs to export the interface:

```wit
package my-username:my-bot@0.1.0;

world component {
  import asterai:host/api@1.0.0;
  export asterai:discord-message-listener/incoming-handler@0.1.0;
}
```

### Message flow

```
Discord Gateway (WebSocket)
  │
  │  asterai:host-ws manages the connection
  │
  ▼
┌──────────────────────────────────┐
│ asterai:discord-message-listener │
│                                  │
│  on-message (from host-ws)       │
│    ├─► call-component-function   │
│    │   my-username:my-bot        │
│    └─► call-component-function   │
│        my-username:my-logger     │
└──────────────────────────────────┘
```
