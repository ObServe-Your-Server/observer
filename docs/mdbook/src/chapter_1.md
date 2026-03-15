# Observer

Observer is a lightweight agent that runs on your server and streams system metrics to a central endpoint.

## Getting Started

Install the agent with a single command:

```sh
curl -fsSL https://raw.githubusercontent.com/ObServe-Your-Server/observer/main/setup/deploy.sh | sudo bash
```

Then check it's running:

```sh
systemctl status observer
journalctl -u observer -f
```

## Configuration

The config lives at `/etc/observer/observer.toml`. Here's a minimal example:

```toml
[server]
base_metrics_url  = "https://your-server.com/api/metrics"
api_key           = "your-api-key"

[intervals]
metric_secs    = 5
speedtest_secs = 300
```

### Config Options

| Key | Default | Description |
|-----|---------|-------------|
| `base_metrics_url` | — | Endpoint to POST metrics to |
| `api_key` | — | Authentication key |
| `metric_secs` | `5` | How often to collect metrics (2–60s) |
| `speedtest_secs` | `300` | How often to run a speedtest (60–86400s) |
| `docker_secs` | `10` | Docker container poll interval |

## What Gets Collected

- **CPU** — usage percentage per core
- **RAM** — used / total memory
- **Storage** — disk usage per mount
- **Uptime** — system uptime in seconds
- **Speedtest** — download, upload, ping via Cloudflare
- **Docker** — per-container CPU and memory stats

> **Note:** The speedtest runs against Cloudflare's speed test endpoint. It adds some outbound traffic — tune `speedtest_secs` if needed.

## Useful Commands

```sh
systemctl restart observer          # restart the agent
systemctl stop observer             # stop the agent
journalctl -u observer --since "1 hour ago"   # recent logs
OBSERVER_LOG_LEVEL=debug /usr/local/bin/observer  # debug mode
```

## Status Indicators

When viewing metrics in the dashboard, servers show one of three states:

- Online — agent is connected and sending data
- Offline — no data received recently
- Connecting — agent started, waiting for first ping

---

See the next section for deployment details.
