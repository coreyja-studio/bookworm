# Eyes coverage

Bookworm publishes `bookworm-operations`, a seven-day dashboard for request
volume, routes, latency, errors, and weekly reading-email outcomes. Fly sets
`EYES_TRANSPORT=batching`; the deployment passes its exact Git SHA into the
Rust build. A boot-stable process identity accompanies the manifest and
telemetry.

Coverage is passive. The manifest declares no external HTTP monitors and no
process-availability expectations. Bookworm's web service is exposed through
Tailscale; recurring probes are unnecessary. Quiet weekdays are normal.
Continuous process health is not asserted: the current runtime still needs
worker supervision and HTTP shutdown improvements before reliable heartbeats
are added.

`CRON_DISABLED=true` omits the weekly cron from both the manifest and runtime.
When enabled, the worker and manifest share `America/New_York`; Eyes receives
`CRON_TZ=America/New_York 0 0 18 * * Sun *` for Sunday at 6 p.m. Eastern. This
requires Eyes' timezone-aware schedule parser. The email runs directly in the
cron worker, not as a registered background job.

Each real email attempt creates a `bookworm.weekly_email` trace. Events are:

| event_type | Meaning |
| --- | --- |
| `bookworm.email_started` | An email attempt began. |
| `bookworm.email_skipped` | No reads this week; no SMTP configuration or call needed. |
| `bookworm.smtp_accepted` | SMTP accepted one recipient's message; includes recipient index/count. |
| `bookworm.email_completed` | All recipients' messages were accepted. |
| `bookworm.email_failed` | The attempt failed; the error chain belongs to the same trace. |

SMTP acceptance does not establish inbox delivery. Sends remain sequential
and stop on the first failure. A partial attempt can have acceptance events
and then a failure without completion. The cron worker can retry failed
attempts, so acceptance counts are sends, not unique delivered emails. Empty
recipient lists are configuration errors instead of false successful sends.
New telemetry fields exclude addresses and message contents.

Build the TypeScript assets before Rust (`pnpm install --frozen-lockfile`,
`pnpm run build`); Rust embeds files from `ts/dist`. Database tests use local
PostgreSQL. Email tests use a stub transport and never connect to SMTP.


The container starts Bookworm independently of Tailscale login and HTTPS setup.
An expired node key or rejected `TS_AUTHKEY` leaves the private web UI
unavailable, while the email cron and Eyes initialization can proceed. Tailnet
setup retries every 30 seconds; each login waits at most 30 seconds. It issues
no HTTP probes to the app. Replace `TS_AUTHKEY` in Fly and let the secret update
restart the machine when reauthentication is needed. The application remains
the entrypoint process, preserving signal delivery and its exit status.

`python3 -B -m unittest discover -s tests -p startup_test.py` exercises rejected
and blocked login, eventual login/HTTPS recovery, SIGTERM and app exit status
using local stand-ins. It does not contact Fly, Tailscale, HTTP or SMTP.
