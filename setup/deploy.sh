#!/bin/bash
set -e

# your GitHub repo
REPO="ObServe-Your-Server/observer"

# download the latest binary directly from GitHub Releases
# GitHub redirects /latest/download/ to whatever the newest release is
curl -L "https://github.com/$REPO/releases/latest/download/observer" -o /usr/local/bin/observer
chmod +x /usr/local/bin/observer

# copy the service file
cp ./observer.service /etc/systemd/system/observer.service

# reload systemd so it picks up the new service file
systemctl daemon-reload

# restart if already running, otherwise start it
systemctl restart observer || systemctl start observer

# print current status
systemctl status observer
