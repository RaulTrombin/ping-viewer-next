#!/bin/bash
set -m
echo "Starting ping viewer next..."
cd app

# Create directories
mkdir -p logs
mkdir -p recordings
mkdir -p firmwares

# If the mounted firmwares directory is empty or missing utils, restore defaults
if [ ! -d "/app/firmwares/utils" ] || [ -z "$(ls -A /app/firmwares/utils 2>/dev/null)" ]; then
    echo "Initializing firmwares directory with default files..."
    cp -rn /app/firmwares_default/* /app/firmwares/ 2>/dev/null || true
fi

# Set permissions
chmod -R 755 /app/logs
chmod -R 755 /app/recordings
chmod -R 755 /app/firmwares
chown -R pingviewer:pingviewer /app/logs
chown -R pingviewer:pingviewer /app/recordings
chown -R pingviewer:pingviewer /app/firmwares

su pingviewer -c "./ping-viewer-next --enable-auto-create --rest-server 0.0.0.0:6060"