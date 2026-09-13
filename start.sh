#!/bin/sh
set -e

app_dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)

# The persistent daemon reconnects to the tailnet independently of Bookworm.
"$app_dir/tailscaled" --state=/data/tailscale/tailscaled.state --socket=/var/run/tailscale/tailscaled.sock &

# Tailnet access serves the private web UI. A rejected/expired login must not
# prevent the email cron and telemetry exporter from starting. Retry setup
# without probing the HTTP app; Fly's init cleans up children when it exits.
(
    until "$app_dir/tailscale" up --timeout=30s --hostname=bookworm --authkey="${TS_AUTHKEY}"; do
        echo "Bookworm tailnet login unavailable; retrying in 30 seconds" >&2
        sleep 30
    done

    until "$app_dir/tailscale" serve --bg 3000; do
        echo "Bookworm tailnet HTTPS setup unavailable; retrying in 30 seconds" >&2
        sleep 30
    done

    echo "Bookworm tailnet HTTPS configured"
) &

# Preserve direct signal delivery and the application's exit status.
exec "$app_dir/bookworm"
