#!/bin/bash
set -e

# Re-execute with sudo if not running as root
if [ "$EUID" -ne 0 ]; then
    exec sudo -E bash "$0" "$@"
fi

REPO="ObServe-Your-Server/observer"
CONFIG_DIR="/etc/observer"
CONFIG_PATH="$CONFIG_DIR/observer.toml"

# ─────────────────────────────────────────────────────────────────────────────
# INIT SYSTEM DETECTION
# ─────────────────────────────────────────────────────────────────────────────
detect_init() {
    if command -v systemctl >/dev/null 2>&1 && systemctl --version >/dev/null 2>&1; then
        echo "systemd"
    elif command -v rc-service >/dev/null 2>&1 || [ -f /sbin/openrc-run ]; then
        echo "openrc"
    else
        echo "unknown"
    fi
}
INIT_SYSTEM=$(detect_init)

svc_stop()    {
    if [ "$INIT_SYSTEM" = "systemd" ]; then
        systemctl stop observer
    else
        rc-service observer stop 2>/dev/null || true
    fi
}
svc_is_active() {
    if [ "$INIT_SYSTEM" = "systemd" ]; then
        systemctl is-active --quiet observer
    else
        rc-service observer status 2>/dev/null | grep -q started
    fi
}
svc_enable_start() {
    if [ "$INIT_SYSTEM" = "systemd" ]; then
        systemctl daemon-reload
        systemctl enable observer
        systemctl restart observer 2>/dev/null || systemctl start observer
    else
        rc-update add observer default 2>/dev/null || true
        rc-service observer stop 2>/dev/null || true
        sleep 2
        rm -f /run/observer.pid
        : > /var/log/observer.log
        rc-service observer start
    fi
}
svc_disable_stop() {
    if [ "$INIT_SYSTEM" = "systemd" ]; then
        systemctl stop observer 2>/dev/null || true
        systemctl disable observer 2>/dev/null || true
        systemctl daemon-reload
    else
        rc-service observer stop 2>/dev/null || true
        rc-update del observer default 2>/dev/null || true
    fi
}
svc_status() {
    if [ "$INIT_SYSTEM" = "systemd" ]; then
        systemctl status observer
    else
        rc-service observer status
    fi
}

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
DEFAULT_METRICS_URL="https://watch-tower.observe.vision/v1/ingest"
DEFAULT_COMMANDS_URL="https://watch-tower.observe.vision/v1/commands"
DEFAULT_DOCKER_URL="https://watch-tower.observe.vision/v1/ingest/docker"
DEFAULT_NOTIFIER_URL="https://watch-tower.observe.vision/v1/ingest/notification"
DEFAULT_API_KEY=""
DEFAULT_METRIC_SECS="5"
DEFAULT_COMMAND_POLL_SECS="10"
DEFAULT_SPEEDTEST_SECS="600"
DEFAULT_DOCKER_SECS="10"
DEFAULT_ENABLE_DOCKER_SOCKET="true"

MODE="full"

if [ -f "$CONFIG_PATH" ]; then
    echo "Observer is already installed." >&2
    echo "" >&2
    echo "  u  Update binary only (keep current config, URLs are always reset)" >&2
    echo "  c  Update config and binary" >&2
    echo "  x  Uninstall" >&2
    echo "  n  Cancel" >&2
    echo "" >&2
    while true; do
        printf "Choice [u/c/x/n]: " >&2
        IFS= read -r REPLY </dev/tty
        case "$REPLY" in
            u) MODE="update_only"; break ;;
            c) MODE="full"; break ;;
            x) MODE="uninstall"; break ;;
            n) echo "Cancelled." >&2; exit 0 ;;
            *) echo "  Please enter u, c, x, or n." >&2 ;;
        esac
    done
    echo "" >&2

    if [ "$MODE" = "uninstall" ]; then
        echo "Uninstalling observer..." >&2
        svc_disable_stop
        rm -f /usr/local/bin/observer
        rm -f /etc/systemd/system/observer.service
        rm -f /etc/init.d/observer
        rm -f "$CONFIG_PATH"
        rmdir "$CONFIG_DIR" 2>/dev/null || true
        echo "Observer uninstalled." >&2
        exit 0
    fi

    if [ "$MODE" = "full" ]; then
        # Pre-fill defaults from the existing config (only override if the field exists)
        prefill_str() {
            local val
            val=$(grep "$1" "$CONFIG_PATH" | sed 's/.*= "\(.*\)"/\1/')
            [ -n "$val" ] && echo "$val" || echo "$2"
        }
        prefill_num() {
            local val
            val=$(grep "$1" "$CONFIG_PATH" | sed 's/[^0-9]*\([0-9]*\).*/\1/')
            [ -n "$val" ] && echo "$val" || echo "$2"
        }
        prefill_bool() {
            local val
            val=$(grep "$1" "$CONFIG_PATH" | grep -o 'true\|false')
            [ -n "$val" ] && echo "$val" || echo "$2"
        }

        DEFAULT_API_KEY=$(prefill_str 'api_key' "$DEFAULT_API_KEY")
        DEFAULT_METRIC_SECS=$(prefill_num 'metric_secs' "$DEFAULT_METRIC_SECS")
        DEFAULT_COMMAND_POLL_SECS=$(prefill_num 'command_poll_secs' "$DEFAULT_COMMAND_POLL_SECS")
        DEFAULT_SPEEDTEST_SECS=$(prefill_num 'speedtest_secs' "$DEFAULT_SPEEDTEST_SECS")
        DEFAULT_DOCKER_SECS=$(prefill_num 'docker_secs' "$DEFAULT_DOCKER_SECS")
        DEFAULT_ENABLE_DOCKER_SOCKET=$(prefill_bool 'enable_docker_socket' "$DEFAULT_ENABLE_DOCKER_SOCKET")
    fi
fi

if [ "$MODE" = "full" ]; then
    echo "Press Enter to accept the default shown in brackets." >&2
    echo "" >&2

    METRICS_URL="$DEFAULT_METRICS_URL"
    COMMANDS_URL="$DEFAULT_COMMANDS_URL"

    ask_required "API key" "$DEFAULT_API_KEY"
    API_KEY="$REPLY"

    ask_optional "Metric send interval in seconds (2-60)" "$DEFAULT_METRIC_SECS"
    METRIC_SECS="$REPLY"

    ask_optional "Command poll interval in seconds (2-60)" "$DEFAULT_COMMAND_POLL_SECS"
    COMMAND_POLL_SECS="$REPLY"

    ask_optional "Speedtest interval in seconds (60-86400)" "$DEFAULT_SPEEDTEST_SECS"
    SPEEDTEST_SECS="$REPLY"

    ask_optional "Docker metric interval in seconds (10-60)" "$DEFAULT_DOCKER_SECS"
    DOCKER_SECS="$REPLY"

    ask_optional "Enable Docker socket monitoring (true/false)" "$DEFAULT_ENABLE_DOCKER_SOCKET"
    ENABLE_DOCKER_SOCKET="$REPLY"

    echo "" >&2
fi

# Stop the service before replacing the binary (can't overwrite a running executable)
if svc_is_active; then
    echo "Stopping observer service..." >&2
    svc_stop
fi

echo "Fetching latest release info..." >&2
LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | sed 's/.*"tag_name": "\(.*\)".*/\1/')
echo "Installing version: $LATEST_TAG" >&2

echo "Detecting architecture..." >&2
case "$(uname -m)" in
  x86_64|amd64)   ARCH_SUFFIX="x86_64" ;;
  aarch64|arm64)  ARCH_SUFFIX="aarch64" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac
echo "Downloading observer binary for $ARCH_SUFFIX..." >&2
curl -fsSL "https://github.com/$REPO/releases/latest/download/observer-$ARCH_SUFFIX" -o /tmp/observer
mv /tmp/observer /usr/local/bin/observer
chmod +x /usr/local/bin/observer

echo "Installing service ($INIT_SYSTEM)..." >&2
if [ "$INIT_SYSTEM" = "systemd" ]; then
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/setup/observer.service" \
        -o /etc/systemd/system/observer.service
elif [ "$INIT_SYSTEM" = "openrc" ]; then
    curl -fsSL "https://raw.githubusercontent.com/$REPO/main/setup/observer.openrc" \
        -o /etc/init.d/observer
    chmod +x /etc/init.d/observer
else
    echo "Warning: unknown init system — skipping service installation. Run observer manually." >&2
fi

if [ "$MODE" = "full" ]; then
    echo "Writing config to $CONFIG_PATH..." >&2
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_PATH" <<EOF
[server]
base_metrics_url  = "$METRICS_URL"
base_commands_url = "$COMMANDS_URL"
base_docker_url   = "$DEFAULT_DOCKER_URL"
base_notifier_url = "$DEFAULT_NOTIFIER_URL"
api_key           = "$API_KEY"

[intervals]
metric_secs          = $METRIC_SECS
command_poll_secs    = $COMMAND_POLL_SECS
speedtest_secs       = $SPEEDTEST_SECS
enable_docker_socket = $ENABLE_DOCKER_SOCKET
docker_secs          = $DOCKER_SECS
EOF
    chmod 600 "$CONFIG_PATH"
fi

# update_only: always overwrite URLs to ensure correct production endpoints
if [ "$MODE" = "update_only" ]; then
    echo "Updating URLs in existing config..." >&2
    sed -i "s|base_metrics_url.*|base_metrics_url  = \"$DEFAULT_METRICS_URL\"|" "$CONFIG_PATH"
    sed -i "s|base_commands_url.*|base_commands_url = \"$DEFAULT_COMMANDS_URL\"|" "$CONFIG_PATH"
    sed -i "s|base_docker_url.*|base_docker_url   = \"$DEFAULT_DOCKER_URL\"|" "$CONFIG_PATH"
    sed -i "s|base_notifier_url.*|base_notifier_url = \"$DEFAULT_NOTIFIER_URL\"|" "$CONFIG_PATH"
fi

echo "Enabling and starting observer service..." >&2
svc_enable_start

echo "" >&2
echo "Observer $LATEST_TAG installed successfully!" >&2
svc_status
