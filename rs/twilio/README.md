# asterai:twilio

SMS integration for asterai.
Send and receive SMS messages via the Twilio API
using webhooks.

## Environment Variables

| Variable                             | Required     | Description                                      |
|--------------------------------------|--------------|--------------------------------------------------|
| `TWILIO_ACCOUNT_SID`                 | Yes          | Account SID from Twilio console                  |
| `TWILIO_AUTH_TOKEN`                  | Yes          | Auth token from Twilio console                   |
| `TWILIO_PHONE_NUMBER`                | Yes          | Your Twilio phone number (E.164 format)          |
| `TWILIO_WEBHOOK_URL`                 | For incoming | Public URL for receiving SMS                     |
| `TWILIO_INCOMING_HANDLER_COMPONENTS` | No           | Comma-separated list of consumer component names |

## Setup

### 1. Create a Twilio account

1. Go to [twilio.com](https://www.twilio.com) and sign up.
2. Verify your email and phone number.
3. Twilio gives you a free trial account with a
   trial balance (no credit card needed).

### 2. Get your Account SID and Auth Token

1. In the [Twilio Console](https://console.twilio.com),
   your **Account SID** and **Auth Token** are shown
   on the main dashboard.
2. Click **Show** next to the Auth Token to reveal it.
3. Set them as `TWILIO_ACCOUNT_SID` and
   `TWILIO_AUTH_TOKEN`.

### 3. Get a phone number

1. In the Twilio Console, go to **Phone Numbers** >
   **Manage** > **Buy a number**.
2. Search for a number with SMS capability and click
   **Buy**.
3. Copy the number in E.164 format (e.g. `+15551234567`).
   Set it as `TWILIO_PHONE_NUMBER`.

Trial accounts come with one free number. You can
also use the trial number assigned during signup.

### 4. Configure the webhook (optional)

This is only needed if you want to receive incoming
SMS messages. The component auto-configures the
webhook URL on your phone number during startup.

1. Set `TWILIO_WEBHOOK_URL` to your public webhook URL
   (e.g. `https://your-domain.com/your-username/your-env-name/asterai/twilio/webhook`).
   If running locally, use a tunnel like
   [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/)
   or [ngrok](https://ngrok.com).
2. The component will automatically update the phone
   number's SMS webhook URL when it starts.

### 5. Configure handler components (optional)

If you want to listen for incoming SMS, set
`TWILIO_INCOMING_HANDLER_COMPONENTS` to a
comma-separated list of components that implement
`asterai:twilio/incoming-handler`.

## Trial account limitations

With a trial account you can:
- Send SMS only to verified phone numbers
  (numbers you've confirmed in the console)
- Use one Twilio phone number
- Access a trial balance (~$15)

To send to any number, upgrade your account.
No monthly fees — you pay per message
(~$0.0079/SMS in the US).

## Webhook signature verification

Incoming webhook requests are verified using
Twilio's `X-Twilio-Signature` header (HMAC-SHA1).
If `TWILIO_WEBHOOK_URL` is not set, all incoming
requests are rejected.
