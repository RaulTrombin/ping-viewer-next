#!/bin/bash
set -m
echo "Starting ping viewer next..."
cd /app
./ping-viewer-next --enable-auto-create --rest-server 0.0.0.0:6060
