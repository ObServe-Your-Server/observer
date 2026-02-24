# Observer

A lightweight agent that runs on a server and collects system metrics. It periodically measures CPU usage, RAM, storage, uptime and runs speedtests, then streams the results to a central endpoint.

## Current state

Work in progress. The following is implemented:

- Config loading from environment variables with validation
- Metric collection via sysinfo (CPU, RAM, storage, uptime)
- Speedtest against Cloudflare (download, upload, ping)
- Periodic schedulers for metrics, command polling and speedtest
- Systemd service setup for Linux

Not yet implemented:

- Actually sending metrics to the server
- Command execution from the command buffer
- All-in-one mode

## Deploy

**1. Download the deploy files onto your server:**

```sh
curl -L https://github.com/ObServe-Your-Server/observer/raw/main/setup/observer.service -o observer.service
curl -L https://github.com/ObServe-Your-Server/observer/raw/main/setup/deploy.sh -o deploy.sh
```

**2. Create and adjust the config:**

```sh
mkdir -p /etc/observer
curl -L https://github.com/ObServe-Your-Server/observer/raw/main/.env.example -o /etc/observer/.env
nano /etc/observer/.env
```

All available options and allowed values are documented in `.env.example`.

**3. Run the deploy script:**

```sh
chmod +x deploy.sh
sudo bash deploy.sh
```

**Updating:** just run `sudo bash deploy.sh` again - it pulls the latest binary and restarts the service.

**Useful commands:**

```sh
systemctl status observer     # check if running
systemctl restart observer    # restart
journalctl -u observer -f     # follow logs
```
