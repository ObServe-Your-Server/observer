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
# Everything works the same as deploy.sh, with these differences:
#
#   1. The binary is pulled from the latest prerelease tag (e.g. v1.2.8-pre.1)
#      instead of the latest stable release.
#
#   2. base_server_grpc_url / base_server_http_url / push_notification_url are
#      ALWAYS set to the staging endpoints below. This prevents a staging
#      machine from accidentally pointing at production.
#
#   3. Every install is a clean install. The existing config and the database
#      (including its -wal/-shm files) are ALWAYS removed and a fresh config in
#      the new format is written. Only the API key is loaded from the old
#      config first, so it can be offered as the default.
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
# Mirrors the repo's observer.toml. The three URL values are always written to
# the config, regardless of what was there before (see note 2 above).
# ─────────────────────────────────────────────────────────────────────────────

STAGING_BASE_SERVER_GRPC_URL="https://grpc-watch-tower-dev.observe.vision:42042"
STAGING_BASE_SERVER_HTTP_URL="https://watch-tower-dev.observe.vision"
STAGING_PUSH_NOTIFICATION_URL="https://watch-tower-dev.observe.vision/notifications"

DEFAULT_DB_PATH="$DATA_DIR/observer.db"
DEFAULT_METRICS_RETENTION_HOURS_FULL_RESOLUTION="1"
DEFAULT_METRICS_RETENTION_HOURS_REDUCED_RESOLUTION="24"
DEFAULT_BASE_METRIC_SECS="5"
DEFAULT_SPEEDTEST_SECS="300"
DEFAULT_CONTAINER_METRICS_SECS="10"
DEFAULT_DATA_CLEANUP_JOB_SECS="3600"

# notification_config: *_notify_after = consecutive readings in a new state
# before a notification is sent
DEFAULT_CPU_NOTIFY_AFTER="2"
DEFAULT_CPU_HIGH_PERCENTAGE="90"
DEFAULT_MEMORY_NOTIFY_AFTER="30"
DEFAULT_MEMORY_HIGH_PERCENTAGE="90"
DEFAULT_DISK_NOTIFY_AFTER="15"
DEFAULT_DISK_HIGH_PERCENTAGE="90"

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

# Removes a sqlite database file together with its sidecar files.
# Only absolute paths are touched.
remove_db() {
    case "$1" in
        /*) rm -f "$1" "$1-wal" "$1-shm" "$1-journal" ;;
    esac
}

echo "=== Observer Prerelease Installer ===" >&2
echo "" >&2

# Try to load the existing config first. Works for the old and the new config
# format. Only the API key is reused, the database path is remembered so the
# old database can be removed even if it lived at a custom location.
MODE=""
DEFAULT_API_KEY=""
OLD_DB_PATH=""
if [ -f "$CONFIG_PATH" ]; then
    DEFAULT_API_KEY=$(grep -m1 '^api_key' "$CONFIG_PATH" | sed 's/.*= *"\(.*\)".*/\1/')
    OLD_DB_PATH=$(grep -m1 '^database_url' "$CONFIG_PATH" | sed 's|.*sqlite://\([^?"]*\).*|\1|')

    echo "Observer is already installed." >&2
    echo "" >&2
    echo "  r  Reinstall (removes the current config AND database, installs a fresh config)" >&2
    echo "  x  Uninstall" >&2
    echo "  n  Cancel" >&2
    echo "" >&2
    while true; do
        printf "Choice [r/x/n]: " >&2
        IFS= read -r REPLY </dev/tty
        case "$REPLY" in
            r) break ;;
            x) MODE="uninstall"; break ;;
            n) echo "Cancelled." >&2; exit 0 ;;
            *) echo "  Please enter r, x, or n." >&2 ;;
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
fi

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

ask_optional "Reduced resolution metrics retention in hours" "$DEFAULT_METRICS_RETENTION_HOURS_REDUCED_RESOLUTION"
METRICS_RETENTION_HOURS_REDUCED_RESOLUTION="$REPLY"
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

echo "Using default gRPC server URL:       $STAGING_BASE_SERVER_GRPC_URL" >&2
echo "Using default HTTP server URL:       $STAGING_BASE_SERVER_HTTP_URL" >&2
echo "Using default push notification URL: $STAGING_PUSH_NOTIFICATION_URL" >&2
echo "Using default metric interval:       ${DEFAULT_BASE_METRIC_SECS}s" >&2
echo "Using default speedtest interval:    ${DEFAULT_SPEEDTEST_SECS}s" >&2
echo "Using default container interval:    ${DEFAULT_CONTAINER_METRICS_SECS}s" >&2
echo "Notify after (readings): CPU ${DEFAULT_CPU_NOTIFY_AFTER}, memory ${DEFAULT_MEMORY_NOTIFY_AFTER}, disk ${DEFAULT_DISK_NOTIFY_AFTER}" >&2
echo "" >&2

# Stop the service before replacing the binary (can't overwrite a running executable)
if svc_is_active; then
    echo "Stopping observer service..." >&2
    svc_stop
fi

# Always start clean: remove the old config and database
echo "Removing old config and database..." >&2
rm -f "$CONFIG_PATH"
remove_db "$OLD_DB_PATH"
remove_db "$DEFAULT_DB_PATH"
remove_db "$DB_PATH"

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

echo "Writing config to $CONFIG_PATH..." >&2
mkdir -p "$CONFIG_DIR"
mkdir -p "$(dirname "$DB_PATH")"
cat > "$CONFIG_PATH" <<EOF
[client_config]
base_server_grpc_url               = "$STAGING_BASE_SERVER_GRPC_URL"
base_server_http_url               = "$STAGING_BASE_SERVER_HTTP_URL"
push_notification_url              = "$STAGING_PUSH_NOTIFICATION_URL"
api_key                            = "$API_KEY"
enable_container_metrics_collector = $ENABLE_DOCKER_SOCKET

[interval_config]
base_metric_secs       = $DEFAULT_BASE_METRIC_SECS
speedtest_secs         = $DEFAULT_SPEEDTEST_SECS
enable_docker_socket   = $ENABLE_DOCKER_SOCKET
container_metrics_secs = $DEFAULT_CONTAINER_METRICS_SECS
data_cleanup_job_secs  = $DEFAULT_DATA_CLEANUP_JOB_SECS

[notification_config]
notification_way = "push_notification"

enable_cpu_notification = true
cpu_notify_after        = $DEFAULT_CPU_NOTIFY_AFTER
cpu_high_percentage     = $DEFAULT_CPU_HIGH_PERCENTAGE

enable_memory_notification = true
memory_notify_after        = $DEFAULT_MEMORY_NOTIFY_AFTER
memory_high_percentage     = $DEFAULT_MEMORY_HIGH_PERCENTAGE

enable_disk_notification = true
disk_notify_after        = $DEFAULT_DISK_NOTIFY_AFTER
disk_high_percentage     = $DEFAULT_DISK_HIGH_PERCENTAGE

enable_container_socket_notifications     = false
notify_on_high_container_socket_usage     = true
container_socket_high_cpu_usage_percent   = 90
container_socket_low_cpu_usage_percent    = 70
container_socket_notify_on_container_down = true

[storage_config]
database_url                               = "$DATABASE_URL"
metrics_retention_hours_full_resolution    = $DEFAULT_METRICS_RETENTION_HOURS_FULL_RESOLUTION
metrics_retention_hours_reduced_resolution = $METRICS_RETENTION_HOURS_REDUCED_RESOLUTION
EOF
chmod 600 "$CONFIG_PATH"

echo "Enabling and starting observer service..." >&2
svc_enable_start

echo "" >&2
echo "Observer prerelease $LATEST_TAG installed successfully!" >&2
svc_status
