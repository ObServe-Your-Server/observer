# Observer

A lightweight agent that runs on a server and collects system metrics. It periodically measures CPU usage, RAM, storage, uptime and runs speedtests, then streams the results to a central endpoint.

## Deploy

**One-liner install (recommended):**

```sh
curl -fsSL https://install.observe.vision | sudo bash
```

The script will interactively ask for your API key, then download the binary, 
install the systemd service, and write the config to `/etc/observer/observer.toml`.

# IMPORTANT: Updating / Wrong API Key

Run the installer again, it will detect the existing installation and prompt you to update the config 
(pre-filled with current values), fix a wrong API key, and restart the service (should happen automatically after the 
script finishes):

```sh
curl -fsSL https://install.observe.vision | sudo bash
```

**Useful commands:**

```sh
systemctl status observer     # check if running
systemctl restart observer    # restart
journalctl -u observer -f     # follow logs
```

# IMPORTANT: The config

The config lays in `/etc/observer/observer.toml`. There the api key can also be viewed anc 
changed. After a change please restart the service:

```sh
sudo nano /etc/observer/observer.toml     # edit config

sudo systemctl restart observer           # restart to apply changes

journalctl -u observer -f                 # follow logs to check if it works
```

When something doesnt work and you run into issues. Please feel free to write us a mail to **mail@observe.vision**.
We will reply as soon as possible and look into it.

---
# Manual deployment

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
