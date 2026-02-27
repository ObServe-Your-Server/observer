# Observer

A lightweight agent that runs on a server and collects system metrics. It periodically measures CPU usage, RAM, storage, uptime and runs speedtests, then streams the results to a central endpoint.

## Current state

Work in progress. The following is implemented:

- Config loading from a TOML file with validation
- Metric collection via sysinfo (CPU, RAM, storage, uptime)
- Speedtest against Cloudflare (download, upload, ping)
- Sending metrics to the server via HTTP POST
- Periodic schedulers for metrics and speedtest
- Systemd service setup for Linux

Not yet implemented:

- Command execution from the command buffer

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

## Manual deployment

Use this if you want to deploy without the deploy script, or on a system where the script does not work.

**1. Build the binary:**

```sh
cargo build --release
```

**2. Place the binary:**

```sh
sudo cp target/release/observer /usr/local/bin/observer
sudo chmod +x /usr/local/bin/observer
```

**3. Create the config directory and place your config:**

```sh
sudo mkdir -p /etc/observer
sudo cp observer.toml.example /etc/observer/observer.toml
sudo nano /etc/observer/observer.toml   # fill in your server URL and API key
```

**4. Install and enable the systemd service:**

```sh
sudo cp setup/observer.service /etc/systemd/system/observer.service
sudo systemctl daemon-reload
sudo systemctl enable observer
sudo systemctl start observer
```

**Starting and stopping:**

```sh
sudo systemctl start observer    # start
sudo systemctl stop observer     # stop
sudo systemctl restart observer  # restart
```

**Checking logs:**

```sh
journalctl -u observer -f                         # follow live logs
journalctl -u observer --since "1 hour ago"       # last hour
OBSERVER_LOG_LEVEL=debug /usr/local/bin/observer  # run manually with debug output
```
