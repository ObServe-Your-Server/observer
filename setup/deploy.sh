#!/bin/bash
set -e

# Re-execute with sudo if not running as root
if [ "$EUID" -ne 0 ]; then
    exec sudo -E bash "$0" "$@"
fi

REPO="ObServe-Your-Server/observer"
CONFIG_DIR="/etc/observer"
CONFIG_PATH="$CONFIG_DIR/observer.toml"

# Prompts go to stderr (never piped, always reaches the terminal).
# Input is read from /dev/tty (the controlling terminal directly).
ask_required() {
    local label="$1"
    local default="$2"
    while true; do
        if [ -n "$default" ]; then
            printf "%s [%s]: " "$label" "$default" >&2
        else
            printf "%s: " "$label" >&2
        fi
        IFS= read -r REPLY </dev/tty
        REPLY="${REPLY:-$default}"
        [ -n "$REPLY" ] && break
        echo "  This field is required." >&2
    done
}

ask_optional() {
    local label="$1"
    local default="$2"
    printf "%s [%s]: " "$label" "$default" >&2
    IFS= read -r REPLY </dev/tty
    REPLY="${REPLY:-$default}"
}

echo "=== Observer Installer ===" >&2
echo "" >&2

# Load existing config values as defaults if already installed
DEFAULT_METRICS_URL=""
DEFAULT_COMMANDS_URL=""
DEFAULT_METRIC_SECS="5"
DEFAULT_COMMAND_POLL_SECS="5"
DEFAULT_SPEEDTEST_SECS="300"

if [ -f "$CONFIG_PATH" ]; then
    echo "Observer is already installed. This will overwrite the existing config at $CONFIG_PATH." >&2
    printf "Continue? [y/N]: " >&2
    IFS= read -r REPLY </dev/tty
    case "$REPLY" in
        [yY][eE][sS]|[yY]) ;;
        *) echo "Aborted." >&2; exit 0 ;;
    esac
    echo "" >&2

    # Pre-fill defaults from the existing config (except api_key)
    DEFAULT_METRICS_URL=$(grep 'base_metrics_url' "$CONFIG_PATH" | sed 's/.*= "\(.*\)"/\1/')
    DEFAULT_COMMANDS_URL=$(grep 'base_commands_url' "$CONFIG_PATH" | sed 's/.*= "\(.*\)"/\1/')
    DEFAULT_METRIC_SECS=$(grep 'metric_secs' "$CONFIG_PATH" | sed 's/[^0-9]*\([0-9]*\).*/\1/')
    DEFAULT_COMMAND_POLL_SECS=$(grep 'command_poll_secs' "$CONFIG_PATH" | sed 's/[^0-9]*\([0-9]*\).*/\1/')
    DEFAULT_SPEEDTEST_SECS=$(grep 'speedtest_secs' "$CONFIG_PATH" | sed 's/[^0-9]*\([0-9]*\).*/\1/')
fi

echo "Press Enter to accept the default shown in brackets." >&2
echo "" >&2

ask_required "Metrics ingestion URL" "$DEFAULT_METRICS_URL"
METRICS_URL="$REPLY"

ask_required "Commands polling URL" "$DEFAULT_COMMANDS_URL"
COMMANDS_URL="$REPLY"

ask_required "API key" ""
API_KEY="$REPLY"

ask_optional "Metric send interval in seconds (2-60)" "$DEFAULT_METRIC_SECS"
METRIC_SECS="$REPLY"

ask_optional "Command poll interval in seconds (2-60)" "$DEFAULT_COMMAND_POLL_SECS"
COMMAND_POLL_SECS="$REPLY"

ask_optional "Speedtest interval in seconds (60-86400)" "$DEFAULT_SPEEDTEST_SECS"
SPEEDTEST_SECS="$REPLY"

echo "" >&2

# Stop the service before replacing the binary (can't overwrite a running executable)
if systemctl is-active --quiet observer; then
    echo "Stopping observer service..." >&2
    systemctl stop observer
fi

echo "Fetching latest release info..." >&2
LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')
echo "Installing version: $LATEST_TAG" >&2

echo "Downloading observer binary..." >&2
curl -fsSL "https://github.com/$REPO/releases/latest/download/observer" -o /tmp/observer
mv /tmp/observer /usr/local/bin/observer
chmod +x /usr/local/bin/observer

echo "Installing systemd service..." >&2
curl -fsSL "https://raw.githubusercontent.com/$REPO/main/setup/observer.service" \
    -o /etc/systemd/system/observer.service

echo "Writing config to $CONFIG_PATH..." >&2
mkdir -p "$CONFIG_DIR"
cat > "$CONFIG_PATH" <<EOF
[server]
base_metrics_url  = "$METRICS_URL"
base_commands_url = "$COMMANDS_URL"
api_key           = "$API_KEY"

[intervals]
metric_secs       = $METRIC_SECS
command_poll_secs = $COMMAND_POLL_SECS
speedtest_secs    = $SPEEDTEST_SECS
EOF

chmod 600 "$CONFIG_PATH"

echo "Enabling and starting observer service..." >&2
systemctl daemon-reload
systemctl enable observer
systemctl restart observer 2>/dev/null || systemctl start observer

echo "" >&2
echo "Observer $LATEST_TAG installed successfully!" >&2
systemctl status observer
