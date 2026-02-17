# asterai:blitzortung

Real-time lightning detection for asterai via
the Blitzortung community network. Connects to
the Blitzortung WebSocket API, filters strikes
by distance from a configured location, and
dispatches to handler components.

## Environment Variables

| Variable                                | Required | Default     | Description                          |
|-----------------------------------------|----------|-------------|--------------------------------------|
| `BLITZORTUNG_INCOMING_HANDLER_COMPONENTS` | Yes      | --          | Comma-separated handler components   |
| `BLITZORTUNG_CENTER_LAT`               | No       | `-35.1082`  | Center latitude (Wagga Wagga)        |
| `BLITZORTUNG_CENTER_LON`               | No       | `147.3598`  | Center longitude (Wagga Wagga)       |
| `BLITZORTUNG_RADIUS_KM`                | No       | `50`        | Detection radius in km               |

## How it works

1. On startup, connects to the lightningmaps.org
   WebSocket API with auto-reconnect enabled.
2. Subscribes with a bounding box derived from the
   configured center and radius.
3. For each incoming batch of strikes, calculates
   haversine distance from the configured center.
4. If within radius, dispatches a `strike` record
   to all handler components via
   `incoming-handler/on-strike`.

## Handler interface

Handler components must export
`asterai:blitzortung/incoming-handler@0.1.0`:

```wit
interface incoming-handler {
  use types.{strike};
  on-strike: func(strike: strike);
}
```

The `strike` record contains:

| Field            | Type | Description                  |
|------------------|------|------------------------------|
| `timestamp-secs` | f64  | Unix timestamp in seconds    |
| `lat`            | f64  | Latitude in degrees          |
| `lon`            | f64  | Longitude in degrees         |
| `distance-km`    | f64  | Distance from center in km   |

## Notes

- Blitzortung is for non-commercial use only.
- A single persistent WebSocket connection is
  fine for a server-side agent. Do not have many
  end-user clients connecting directly.
- During quiet weather there may be no strikes.
  During storms the rate can be very high — the
  distance filter keeps dispatches manageable.
- Falls back to live2.lightningmaps.org on
  connection failure.
