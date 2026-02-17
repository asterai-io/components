# Steel

[Steel](https://steel.dev) browser automation component for [asterai](https://asterai.io).

## Setup

Set the `STEEL_API_KEY` environment variable. Get a free API key at [steel.dev](https://steel.dev) (100 browser hours/month on the free plan).

## Functions

### Sessions

- `create-session(options)` — Create a new cloud browser session. Options: `use-proxy`, `solve-captcha`, `block-ads`, `timeout`, `user-agent`, `proxy-url`, `session-id`, `headless`.
- `release-session(session-id)` — Close a session.
- `release-all-sessions()` — Close all active sessions.
- `get-session(session-id)` — Get session details.
- `list-sessions()` — List all active sessions.

### Quick Actions

- `scrape(options)` — Scrape a webpage. Required: `url`. Optional: `formats` (`html`, `markdown`, `readability`, `cleaned_html`), `screenshot`, `pdf`, `use-proxy`, `delay`.
- `screenshot(options)` — Screenshot a webpage. Required: `url`. Optional: `full-page`, `use-proxy`, `delay`.
- `pdf(options)` — Generate a PDF of a webpage. Required: `url`. Optional: `use-proxy`, `delay`.

## Not Yet Implemented

Steel's `computer` endpoint (`POST /v1/sessions/{sessionId}/computer`) is not yet exposed. This endpoint allows full browser control via REST — clicking, typing, scrolling, taking screenshots — which is needed for interactive automation (e.g. filling forms, paginating, logging in). The current component covers scraping and session management only.
