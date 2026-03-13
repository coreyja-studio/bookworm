#!/bin/sh
set -e

# Start tailscaled with state stored on the persistent volume
/app/tailscaled --state=/data/tailscale/tailscaled.state --socket=/var/run/tailscale/tailscaled.sock &

# Wait for tailscaled to be ready
sleep 2

# Bring up tailscale - uses TS_AUTHKEY env var automatically
/app/tailscale up --hostname=bookworm

# Start the application
exec /app/bookworm
