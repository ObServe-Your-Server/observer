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

**One-liner install (recommended):**

```sh
curl -fsSL https://install.observe.vision | sudo bash
```

The script will interactively ask for your server URLs and API key, then download the binary, install the systemd service, and write the config to `/etc/observer/observer.toml`.

**Updating:** run the same one-liner again — it will ask if you want to overwrite the existing config, pre-fill current values as defaults, and restart the service.

**Useful commands:**

```sh
systemctl status observer     # check if running
systemctl restart observer    # restart
journalctl -u observer -f     # follow logs
```

## Manual deployment

Use this if you want to deploy without the install script, or on a system where it does not work.

**1. Build the binary:**

```sh
cargo build --release
```

**2. Place the binary:**

```sh
sudo cp target/release/observer /usr/local/bin/observer
sudo chmod +x /usr/local/bin/observer
```

**3. Create the config:**

```sh
sudo mkdir -p /etc/observer
sudo cp observer.toml.example /etc/observer/observer.toml
sudo nano /etc/observer/observer.toml   # fill in your server URLs and API key
```

The config is read from `/etc/observer/observer.toml`. All available options are documented in `observer.toml.example`.

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
