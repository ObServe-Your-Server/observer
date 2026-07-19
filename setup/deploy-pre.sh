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
#   2. base_server_url / base_notifier_url are ALWAYS overwritten to the
#      staging endpoints below — even when updating an existing install.
#      This prevents a staging machine from accidentally pointing at production.
#
# Config values are NOT loaded from a previously installed config (except the
# API key, which is reused if present). Everything else defaults to the
# values below, which mirror the repo's observer.toml.
# ─────────────────────────────────────────────────────────────────────────────

REPO="ObServe-Your-Server/observer"
CONFIG_DIR="/etc/observer"
CONFIG_PATH="$CONFIG_DIR/observer.toml"
DATA_DIR="/var/lib/observer"

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

# ─────────────────────────────────────────────────────────────────────────────
# STAGING ENDPOINTS & DEFAULTS
# Mirrors the repo's observer.toml. The two URL values are always written to
# the config, regardless of what was there before (see note 2 above).
# ─────────────────────────────────────────────────────────────────────────────

STAGING_BASE_SERVER_URL="https://grpc-watch-tower-dev.observe.vision:42042"
STAGING_BASE_NOTIFIER_URL="none"

DEFAULT_DB_PATH="$DATA_DIR/observer.db"
DEFAULT_METRICS_RETENTION_HOURS="24"
DEFAULT_METRIC_SECS="5"
DEFAULT_SPEEDTEST_SECS="300"
DEFAULT_ENABLE_DOCKER_SOCKET="true"
DEFAULT_DOCKER_SECS="10"

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

# Yes/no prompt. Sets REPLY_YES to "true" or "false".
ask_yes_no() {
    local label="$1"
    local default="$2" # "y" or "n"
    local hint="y/N"
    [ "$default" = "y" ] && hint="Y/n"
    while true; do
        printf "%s [%s]: " "$label" "$hint" >&2
        IFS= read -r REPLY </dev/tty
        REPLY="${REPLY:-$default}"
        case "$REPLY" in
            y|Y|yes|Yes) REPLY_YES="true"; break ;;
            n|N|no|No)   REPLY_YES="false"; break ;;
            *) echo "  Please answer y or n." >&2 ;;
        esac
    done
}

echo "=== Observer Prerelease Installer ===" >&2
echo "" >&2

MODE="full"
DEFAULT_API_KEY=""

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
        echo "Observer uninstalled. Data in $DATA_DIR was left untouched." >&2
        exit 0
    fi

    # Only the API key is reused from an existing install. Every other value
    # (URLs, database location, retention, intervals, docker) is re-derived
    # from the defaults below, not from whatever is currently on disk.
    DEFAULT_API_KEY=$(grep 'api_key' "$CONFIG_PATH" | sed 's/.*= "\(.*\)"/\1/')
fi

if [ "$MODE" = "full" ]; then
    echo "Press Enter to accept the default shown in brackets." >&2
    echo "" >&2

    echo "Paste your API key below." >&2
    ask_required "API key" "$DEFAULT_API_KEY"
    API_KEY="$REPLY"
    echo "" >&2

    echo "Default database location: $DEFAULT_DB_PATH (SQLite)" >&2
    ask_yes_no "Use a custom absolute path instead?" "n"
    if [ "$REPLY_YES" = "true" ]; then
        while true; do
            ask_required "Absolute path to database file" "$DEFAULT_DB_PATH"
            case "$REPLY" in
                /*) DB_PATH="$REPLY"; break ;;
                *) echo "  Path must be absolute (start with /)." >&2 ;;
            esac
        done
    else
        DB_PATH="$DEFAULT_DB_PATH"
    fi
    DATABASE_URL="sqlite://$DB_PATH?mode=rwc"
    echo "" >&2

    ask_optional "Metrics retention time in hours" "$DEFAULT_METRICS_RETENTION_HOURS"
    METRICS_RETENTION_HOURS="$REPLY"
    echo "" >&2

    if [ -S /var/run/docker.sock ] || [ -S /run/docker.sock ]; then
        echo "Detected a Docker socket on this system." >&2
        DOCKER_DEFAULT="y"
    else
        echo "No Docker socket was detected on this system." >&2
        DOCKER_DEFAULT="n"
    fi
    echo "Note: if Docker monitoring is enabled but no Docker socket is found at runtime, Observer will terminate." >&2
    ask_yes_no "Is a Docker socket running that Observer should monitor?" "$DOCKER_DEFAULT"
    ENABLE_DOCKER_SOCKET="$REPLY_YES"
    echo "" >&2

    echo "Using default server URL:          $STAGING_BASE_SERVER_URL" >&2
    echo "Using default notifier URL:        $STAGING_BASE_NOTIFIER_URL" >&2
    echo "Using default metric interval:     ${DEFAULT_METRIC_SECS}s" >&2
    echo "Using default speedtest interval:  ${DEFAULT_SPEEDTEST_SECS}s" >&2
    echo "Using default docker interval:     ${DEFAULT_DOCKER_SECS}s" >&2
    echo "" >&2
fi

# Stop the service before replacing the binary (can't overwrite a running executable)
if svc_is_active; then
    echo "Stopping observer service..." >&2
    svc_stop
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
    mkdir -p "$(dirname "$DB_PATH")"
    cat > "$CONFIG_PATH" <<EOF
[server]
base_server_url   = "$STAGING_BASE_SERVER_URL"
base_notifier_url = "$STAGING_BASE_NOTIFIER_URL"
database_url      = "$DATABASE_URL"
api_key           = "$API_KEY"
metrics_retention_time_hours = $METRICS_RETENTION_HOURS

[intervals]
metric_secs           = $DEFAULT_METRIC_SECS
speedtest_secs         = $DEFAULT_SPEEDTEST_SECS
enable_docker_socket   = $ENABLE_DOCKER_SOCKET
docker_secs            = $DEFAULT_DOCKER_SECS
EOF
    chmod 600 "$CONFIG_PATH"
fi

# update_only: rewrite only the URL fields, leave everything else untouched
if [ "$MODE" = "update_only" ]; then
    echo "Updating staging URLs in existing config..." >&2
    sed -i "s|base_server_url.*|base_server_url   = \"$STAGING_BASE_SERVER_URL\"|" "$CONFIG_PATH"
    sed -i "s|base_notifier_url.*|base_notifier_url = \"$STAGING_BASE_NOTIFIER_URL\"|" "$CONFIG_PATH"
fi

echo "Enabling and starting observer service..." >&2
svc_enable_start

echo "" >&2
echo "Observer prerelease $LATEST_TAG installed successfully!" >&2
svc_status
