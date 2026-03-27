#!/bin/bash
set -e

# Re-execute with sudo if not running as root
if [ "$EUID" -ne 0 ]; then
    exec sudo -E bash "$0" "$@"
fi

# ─────────────────────────────────────────────────────────────────────────────
# PRERELEASE INSTALLER
#
# This script installs the latest *prerelease* build of observer from GitHub.
# Everything works the same as deploy.sh, with two differences:
#
#   1. The binary is pulled from the latest prerelease tag (e.g. v1.2.8-pre.1)
#      instead of the latest stable release.
#
#   2. The four metrics/commands/docker/notifier URLs are ALWAYS overwritten
#      to the staging endpoints below — even when updating an existing install.
#      This prevents a staging machine from accidentally pointing at production.
#
# To adjust timeouts or intervals, just change the defaults in the section
# marked "STAGING ENDPOINTS & DEFAULTS" below.
# ─────────────────────────────────────────────────────────────────────────────

REPO="ObServe-Your-Server/observer"
CONFIG_DIR="/etc/observer"
CONFIG_PATH="$CONFIG_DIR/observer.toml"

# ─────────────────────────────────────────────────────────────────────────────
# STAGING ENDPOINTS & DEFAULTS
# Change these to point at your staging / prerelease environment.
# The four URL values are always written to the config, regardless of what
# was there before — this is intentional (see note 2 above).
# ─────────────────────────────────────────────────────────────────────────────

STAGING_METRICS_URL="https://staging.observe.vision/v1/ingest"
STAGING_COMMANDS_URL="https://staging.observe.vision/v1/commands"
STAGING_DOCKER_URL="https://staging.observe.vision/v1/ingest/docker"
STAGING_NOTIFIER_URL="https://staging.observe.vision/v1/ingest/notification"

DEFAULT_METRIC_SECS="5"
DEFAULT_COMMAND_POLL_SECS="10"
DEFAULT_SPEEDTEST_SECS="600"
DEFAULT_DOCKER_SECS="10"
DEFAULT_ENABLE_DOCKER_SOCKET="true"

# ─────────────────────────────────────────────────────────────────────────────

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

echo "=== Observer Prerelease Installer ===" >&2
echo "" >&2

MODE="full"
DEFAULT_API_KEY=""

if [ -f "$CONFIG_PATH" ]; then
    echo "Observer is already installed." >&2
    echo "" >&2
    echo "  u  Update binary only (keep current config, except URLs — those are always overwritten)" >&2
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
        systemctl stop observer 2>/dev/null || true
        systemctl disable observer 2>/dev/null || true
        rm -f /usr/local/bin/observer
        rm -f /etc/systemd/system/observer.service
        rm -f "$CONFIG_PATH"
        rmdir "$CONFIG_DIR" 2>/dev/null || true
        systemctl daemon-reload
        echo "Observer uninstalled." >&2
        exit 0
    fi

    # Load existing config values as defaults for the interval/key fields.
    # Note: URL fields are intentionally NOT loaded — they are always reset
    # to the staging endpoints defined above.
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

if [ "$MODE" = "full" ]; then
    echo "Press Enter to accept the default shown in brackets." >&2
    echo "" >&2

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
if systemctl is-active --quiet observer; then
    echo "Stopping observer service..." >&2
    systemctl stop observer
fi

# Fetch the latest prerelease tag.
# The GitHub releases API returns releases sorted by creation date newest first.
# We filter to prereleases only and take the first one.
echo "Fetching latest prerelease info..." >&2
LATEST_TAG=$(curl -fsSL "https://api.github.com/repos/$REPO/releases" \
    | grep -B5 '"prerelease": true' \
    | grep '"tag_name"' \
    | head -1 \
    | sed 's/.*"tag_name": "\(.*\)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
    echo "No prerelease found for $REPO. Have you published one yet?" >&2
    exit 1
fi
echo "Installing prerelease version: $LATEST_TAG" >&2

echo "Detecting architecture..." >&2
case "$(uname -m)" in
  x86_64|amd64)   ARCH_SUFFIX="x86_64" ;;
  aarch64|arm64)  ARCH_SUFFIX="aarch64" ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac
echo "Downloading observer binary for $ARCH_SUFFIX..." >&2
curl -fsSL "https://github.com/$REPO/releases/download/$LATEST_TAG/observer-$ARCH_SUFFIX" \
    -o /tmp/observer
mv /tmp/observer /usr/local/bin/observer
chmod +x /usr/local/bin/observer

echo "Installing systemd service..." >&2
curl -fsSL "https://raw.githubusercontent.com/$REPO/main/setup/observer.service" \
    -o /etc/systemd/system/observer.service

if [ "$MODE" = "full" ]; then
    echo "Writing config to $CONFIG_PATH..." >&2
    mkdir -p "$CONFIG_DIR"
    cat > "$CONFIG_PATH" <<EOF
[server]
base_metrics_url  = "$STAGING_METRICS_URL"
base_commands_url = "$STAGING_COMMANDS_URL"
base_docker_url   = "$STAGING_DOCKER_URL"
base_notifier_url = "$STAGING_NOTIFIER_URL"
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

# update_only: rewrite only the URL fields, leave everything else untouched
if [ "$MODE" = "update_only" ]; then
    echo "Updating staging URLs in existing config..." >&2
    sed -i "s|base_metrics_url.*|base_metrics_url  = \"$STAGING_METRICS_URL\"|" "$CONFIG_PATH"
    sed -i "s|base_commands_url.*|base_commands_url = \"$STAGING_COMMANDS_URL\"|" "$CONFIG_PATH"
    sed -i "s|base_docker_url.*|base_docker_url   = \"$STAGING_DOCKER_URL\"|" "$CONFIG_PATH"
    sed -i "s|base_notifier_url.*|base_notifier_url = \"$STAGING_NOTIFIER_URL\"|" "$CONFIG_PATH"
fi

echo "Enabling and starting observer service..." >&2
systemctl daemon-reload
systemctl enable observer
systemctl restart observer 2>/dev/null || systemctl start observer

echo "" >&2
echo "Observer prerelease $LATEST_TAG installed successfully!" >&2
systemctl status observer
