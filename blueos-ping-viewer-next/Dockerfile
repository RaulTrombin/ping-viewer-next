FROM alpine:3.18
RUN apk add --no-cache bash shadow wget unzip

COPY ./blueos-ping-viewer-next/files/ping-viewer-next.* /
COPY ./blueos-ping-viewer-next/files/entrypoint.sh /

RUN mkdir -p /app
RUN chmod +x /entrypoint.sh && \
    if [ "$(uname -m)" = "aarch64" ]; then \
        cp /ping-viewer-next.aarch64 /app/ping-viewer-next; \
    elif [ "$(uname -m)" = "x86_64" ]; then \
        cp /ping-viewer-next.x86_64 /app/ping-viewer-next; \
    else \
        cp /ping-viewer-next.armv7 /app/ping-viewer-next; \
    fi && \
    chmod +x  /app/ping-viewer-next && \
    rm /ping-viewer-next.*
RUN set -eux; \
    mkdir -p /app/firmwares/utils; \
		if [ "$(uname -m)" = "aarch64" ]; then \
			ARCH_TRIPLE="aarch64-unknown-linux-musl"; \
		elif [ "$(uname -m)" = "x86_64" ]; then \
			ARCH_TRIPLE="x86_64-unknown-linux-musl"; \
		else \
			ARCH_TRIPLE="armv7-unknown-linux-musleabihf"; \
		fi; \
		PING_BOOT_URL="https://github.com/bluerobotics/ping360-bootloader/releases/download/0.0.0/ping360-bootloader-${ARCH_TRIPLE}.zip"; \
		STM32_URL="https://github.com/bluerobotics/stm32flash-code/releases/download/0.0.0-br/stm32flash-${ARCH_TRIPLE}.zip"; \
		cd /tmp; \
		echo "Attempting to download ping360-bootloader from: ${PING_BOOT_URL}"; \
		if wget -q -O ping360-bootloader.zip "${PING_BOOT_URL}"; then \
			mkdir -p /tmp/ping360-bootloader-extract; \
			unzip -q -j /tmp/ping360-bootloader.zip -d /tmp/ping360-bootloader-extract || true; \
			PING_BOOT_BIN="$(find /tmp/ping360-bootloader-extract -maxdepth 1 -type f -name 'ping360-bootloader*' | head -n1)"; \
			if [ -n "${PING_BOOT_BIN}" ]; then \
				mv "${PING_BOOT_BIN}" /app/firmwares/utils/ping360-bootloader; \
			else \
				echo "ping360-bootloader binary not found in archive, skipping."; \
			fi; \
		else \
			echo "Failed to download ${PING_BOOT_URL}, skipping."; \
		fi; \
		echo "Attempting to download stm32flash from: ${STM32_URL}"; \
		if wget -q -O stm32flash.zip "${STM32_URL}"; then \
			mkdir -p /tmp/stm32flash-extract; \
			unzip -q -j /tmp/stm32flash.zip -d /tmp/stm32flash-extract || true; \
			STM32_BIN="$(find /tmp/stm32flash-extract -maxdepth 1 -type f -name 'stm32flash*' | head -n1)"; \
			if [ -n "${STM32_BIN}" ]; then \
				mv "${STM32_BIN}" /app/firmwares/utils/stm32flash; \
			else \
				echo "stm32flash binary not found in archive, skipping."; \
			fi; \
		else \
			echo "Failed to download ${STM32_URL}, skipping."; \
		fi; \
	chmod a+x /app/firmwares/utils/ping360-bootloader /app/firmwares/utils/stm32flash || true; \
	rm -rf /tmp/ping360-bootloader.zip /tmp/stm32flash.zip /tmp/ping360-bootloader-extract /tmp/stm32flash-extract || true
LABEL version="1.0.0-beta.6"

RUN addgroup -g 1000 pingviewer && adduser -G pingviewer -u 1000 pingviewer -D

# Add docker configuration
LABEL permissions='{\
  "ExposedPorts": {\
    "6060/tcp": {}\
  },\
  "HostConfig": {\
    "Privileged": true,\
    "NetworkMode": "host",\
    "Binds": [\
       "/usr/blueos/extensions/ping-viewer-next/logs:/app/logs",\
       "/usr/blueos/extensions/ping-viewer-next/recordings:/app/recordings"\
    ]\
  }\
}'
LABEL authors='[\
    {\
        "name": "Raul Victor Trombin",\
        "email": "raulvtrombin@gmail.com"\
    }\
]'
LABEL company='{\
  "about": "Control PingProtocol based hardware using webservices",\
  "name": "Blue Robotics",\
  "email": "support@bluerobotics.com"\
}'
LABEL readme="https://raw.githubusercontent.com/bluerobotics/ping-viewer-next/refs/heads/master/blueos-ping-viewer-next/README.md"
LABEL type="device-integration"
LABEL tags='[\
  "sonar",\
  "ping-protocol"\
]'

ENTRYPOINT ["/entrypoint.sh"]
