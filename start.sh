#!/bin/sh
set -e

# Start tailscaled with state stored on the persistent volume
/app/tailscaled --state=/data/tailscale/tailscaled.state --socket=/var/run/tailscale/tailscaled.sock &

# Wait for tailscaled to be ready
sleep 2

# Bring up tailscale with auth key
/app/tailscale up --hostname=bookworm --authkey="${TS_AUTHKEY}"

# Serve HTTPS on the tailnet, proxying to the app on port 3000
/app/tailscale serve --bg 3000

# Start the application
exec /app/bookworm
