# Installation

## One-liner installer

```sh
curl -fsSL https://install.observe.vision | sudo bash
```

This script detects your init system (systemd or OpenRC) and installs Observer as a managed service.

## Manual installation

Download the latest binary from the releases page and place it in `/usr/local/bin/observer` or a directory
of your choice.
Then place the config file at `/etc/observer/observer.toml` or any other dir if you want:

Example:

```sh
# download and place the bin in there
mkdir -p /usr/local/bin/observer
```


### Create the config dir 

```sh
mkdir -p /etc/observer
touch /etc/observer/observer.toml
chmod 600 /etc/observer/observer.toml
```
In this file you can now place the config:

```sh
[server]
base_metrics_url  = "https://watch-tower.observe.vision/v1/ingest"
base_commands_url = "https://watch-tower.observe.vision/v1/commands"
base_docker_url   = "https://watch-tower.observe.vision/v1/ingest/docker"
base_notifier_url = "https://watch-tower.observe.vision/v1/ingest/notification"
api_key           = "<api-key>"

[intervals]
metric_secs       = 2           # 2–60 seconds
command_poll_secs = 10          # 1–60 seconds
speedtest_secs    = 3600        # 60–86400 seconds (1 hour)
enable_docker_socket = true
docker_secs       = 10          # 10–60 seconds
```
Then it is only a matter of inserting the api key and starting the application with the env for the config file.

```sh
OBSERVER_CONFIG=/etc/observer/observer.toml /usr/local/bin/observer
```

## Compile from source

Requires [Rust](https://rustup.rs) (edition 2024, stable toolchain).

```sh
git clone https://github.com/ObServe-Your-Server/observer.git
cd observer
cargo build --release
cp target/release/observer /usr/local/bin/observer
```

Then the rest of the setup from above also works.
