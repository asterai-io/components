# asterai:lightning-notifier

Lightning notification orchestrator for asterai.
Monitors for lightning near a configured location
and sends proactive alerts via any messaging
component.

## Environment Variables

| Variable           | Required | Default            | Description                        |
|--------------------|----------|--------------------|------------------------------------|
| `NOTIFY_COMPONENT` | Yes      | --                 | Messaging component name           |
| `NOTIFY_RECIPIENT` | Yes      | --                 | Recipient (phone, chat ID, etc.)   |
| `NOTIFY_FUNCTION`  | No       | `api/send-message` | Function to call on the component  |

## Notification rules

### Blitzortung strikes (real-time)
When a lightning strike is detected within range,
sends a notification immediately. Further strikes
are suppressed for 4 hours. After the cooldown,
the next strike triggers a new alert with a count
of strikes since the last notification.

### BOM severe thunderstorm warnings (every 5 min)
Polls BOM for active warnings. When a new severe
thunderstorm warning appears, sends a notification.
Warnings are deduplicated by ID so the same warning
is only notified once.

### BOM daily forecast (every 6 hours)
Polls BOM for the daily forecast. If thunderstorms
are forecast (icon descriptor "storm" or text
mentions "thunderstorm"), sends a notification.
Limited to once per 24 hours.

## Required components

This component requires the following components
in the same environment:

- `asterai:bom` — weather forecast and warning data
  (directly imported via WIT)
- `asterai:blitzortung` — real-time lightning strikes
  (exports incoming-handler for strike events)
- A messaging component (e.g. `asterai:twilio`,
  `asterai:telegram`) — for sending notifications
  (called dynamically via `call-component-function`)

## Setup

### 1. Configure blitzortung

Set `BLITZORTUNG_INCOMING_HANDLER_COMPONENTS` to
include this component:
```
BLITZORTUNG_INCOMING_HANDLER_COMPONENTS=asterai:lightning-notifier
```

### 2. Configure notifications

Choose a messaging component and set the env vars.

**Example with Twilio (SMS):**
```
NOTIFY_COMPONENT=asterai:twilio
NOTIFY_RECIPIENT=+61412345678
```

**Example with Telegram:**
```
NOTIFY_COMPONENT=asterai:telegram
NOTIFY_RECIPIENT=123456789
```

### 3. Run with --allow-dir

State is persisted to disk. Pass `--allow-dir`
when running the environment:
```
asterai env run my-env --allow-dir ~/.lightning-notifier
```

## State

State is stored in `lightning-notifier-state.json`
in the allowed directory:

- `last_strike_notify_secs` — cooldown timestamp
- `strike_count` — strikes since last notification
- `last_forecast_notify_secs` — forecast cooldown
- `notified_warning_ids` — deduplicated warning IDs
- `geohash` — cached BOM geohash (auto-resolved
  from "Wagga Wagga" on first run)

## Notes

- Location is currently hardcoded to Wagga Wagga.
  Blitzortung filtering is configured via its own
  env vars (`BLITZORTUNG_CENTER_LAT`, etc.).
- The component works with any messaging component
  that exports `send-message(content, to) -> string`.
- State is loaded and saved on every handler call
  since WASM instances don't persist in-memory
  state across calls.
