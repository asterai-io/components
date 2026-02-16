# asterai:whatsapp

WhatsApp integration for asterai.
Send and receive messages via the Meta WhatsApp
Cloud API using webhooks.

## Environment Variables

| Variable                               | Required | Description                                      |
|----------------------------------------|----------|--------------------------------------------------|
| `WHATSAPP_ACCESS_TOKEN`                | Yes      | Permanent access token from Meta                 |
| `WHATSAPP_PHONE_NUMBER_ID`             | Yes      | Phone number ID from Meta dashboard              |
| `WHATSAPP_APP_SECRET`                  | Yes      | App Secret from Meta dashboard                   |
| `WHATSAPP_WEBHOOK_VERIFY_TOKEN`        | Yes      | Token you choose for webhook verification        |
| `WHATSAPP_INCOMING_HANDLER_COMPONENTS` | No       | Comma-separated list of consumer component names |

## Setup

### 1. Create a Meta Developer App

1. Go to [developers.facebook.com](https://developers.facebook.com)
   and log in (or create an account).
2. Click **My Apps** > **Create App**.
3. Select **Business** as the app type.
4. Give it a name and click **Create App**.

### 2. Add WhatsApp to your app

1. In your app dashboard, click **Add Product**.
2. Find **WhatsApp** and click **Set Up**.
3. This creates a WhatsApp Business Account (WABA)
   and gives you a test phone number.

### 3. Get your Phone Number ID

1. In the left sidebar, go to **WhatsApp** >
   Phone Numbers.
2. You'll see a **Phone number ID** — copy it.
   Set it as `WHATSAPP_PHONE_NUMBER_ID`. Example: 1234567891234567.
3. (Optional) Click **Add phone number** to register
   your own number instead of using the test number.

### 4. Generate a permanent access token

The token shown on the API Setup page is temporary
(expires in 24h). For a permanent token:

1. Go to [business.facebook.com](https://business.facebook.com)
   > **Settings** > **Users** > **System Users**.
2. Click **Add** to create a system user
   (select **Admin** role).
3. With the system user selected, click on the "..." button and
   "Assign assets". Assign the user to your app. Give it full control
   to your app and WhatsApp account.
4. Click **Generate New Token**, select your app,
   and enable the WhatsApp permissions:
   - `whatsapp_business_messaging`
   - `whatsapp_business_management`
   - `whatsapp_business_manage_events`
5. Copy the token. Set it as `WHATSAPP_ACCESS_TOKEN`.

### 5. Get your App Secret

1. In the Meta Developer Dashboard, go to your app >
   **Settings** > **Basic**.
2. Click **Show** next to **App Secret** and copy it.
   Set it as `WHATSAPP_APP_SECRET`.

This is used to verify that incoming webhook requests
actually came from Meta (via HMAC-SHA256 signature).

### 6. Choose a webhook verify token

Pick any random string (e.g. `openssl rand -hex 16`).
Set it as `WHATSAPP_WEBHOOK_VERIFY_TOKEN`.
Meta uses this to verify your webhook endpoint
with a one-time GET request.

### 7. Configure the webhook in Meta

1. In the Meta Developer Dashboard, go to your app
   and click "Customize the Connect with customers
   through WhatsApp use case", then go to Webhooks.
2. Under **Webhook**, click **Edit**.
3. Enter your public webhook URL as the
   **Callback URL** (e.g.
   `https://your-domain.com/your-username/your-env-name/asterai/whatsapp/webhook`).
   If running locally, use a tunnel like
   [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/)
   or [ngrok](https://ngrok.com).
4. Enter your verify token as the **Verify Token**.
5. Click **Verify and Save**.
6. Under **Webhook fields**, click **Manage** and
   subscribe to **messages**.

### 8. Configure handler components (optional)

If you want to listen for incoming messages, set
`WHATSAPP_INCOMING_HANDLER_COMPONENTS` to a
comma-separated list of components that implement
`asterai:whatsapp/incoming-handler`.

## No business verification required

You can start using the API immediately without
Meta business verification. Without verification
you get:
- 2 phone numbers
- 250 business-initiated messages per 24h
- Unlimited replies to user-initiated conversations
- 1,000 free service conversations per month
