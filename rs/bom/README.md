# asterai:bom

Australian Bureau of Meteorology weather API wrapper
for asterai. Provides forecasts, warnings, and
location search via the undocumented BOM JSON API.

No authentication required.

## Interface

- `search-locations(query) -> list<location>`
  Search by place name or coordinates.
  Returns geohashes needed for other calls.

- `get-daily-forecasts(geohash) -> list<daily-forecast>`
  7-day daily forecasts including temperature,
  rain chance, icon descriptor, and text summaries.

- `get-warnings(geohash) -> list<warning>`
  Active weather warnings (severe thunderstorm,
  flood, etc.) for the location.

## Environment Variables

None.

## Usage

Import `asterai:bom/api@0.1.0` in your component.wit:

```wit
world component {
  import asterai:bom/api@0.1.0;
  // ...
}
```

Then call directly with typed returns:

```rust
use crate::bindings::asterai::bom::api as bom;

let locations = bom::search_locations("Sydney");
let geohash = &locations[0].geohash;
let forecasts = bom::get_daily_forecasts(geohash);
let warnings = bom::get_warnings(geohash);
```

## Storm detection

To detect thunderstorm risk from daily forecasts:
- `icon-descriptor == "storm"`
- `extended-text` contains "thunderstorm"

To detect active severe weather from warnings:
- `warning-type` contains "thunderstorm"

## Notes

- The BOM API is undocumented and not officially
  supported for third-party use. It could change
  without notice.
- No rate limiting is currently enforced, but poll
  respectfully (forecasts every 6h, warnings every
  5min).
- Locations use 7-character geohashes. Use
  `search-locations` to find the geohash for a
  place name.
