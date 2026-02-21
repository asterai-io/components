# asterai:s3

S3 backend for the `asterai:fs` interface.

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `S3_BUCKET` | Yes | S3 bucket name |
| `S3_ACCESS_KEY_ID` | Yes | AWS access key |
| `S3_SECRET_ACCESS_KEY` | Yes | AWS secret key |
| `S3_REGION` | No | AWS region (default: `us-east-1`) |
| `S3_ENDPOINT` | No | Custom endpoint for S3-compatible services (MinIO, R2, etc.) |
| `S3_PREFIX` | No | Root prefix within the bucket |
