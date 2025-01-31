FROM alpine:3.18
RUN apk add --no-cache bash

COPY ./blueos-ping-viewer-next/files/ping-viewer-next.* /
COPY ./blueos-ping-viewer-next/files/entrypoint.sh /

RUN chmod +x /entrypoint.sh && \
    if [ "$(uname -m)" = "aarch64" ]; then \
        cp /ping-viewer-next.aarch64 /ping-viewer-next; \
    elif [ "$(uname -m)" = "x86_64" ]; then \
        cp /ping-viewer-next.x86_64 /ping-viewer-next; \
    else \
        cp /ping-viewer-next.armv7 /ping-viewer-next; \
    fi && \
    chmod +x /ping-viewer-next && \
    rm /ping-viewer-next.*
LABEL version="0.0.0"

# Add docker configuration
LABEL permissions='{\
  "ExposedPorts": {\
    "6060/tcp": {}\
  },\
  "HostConfig": {\
    "Privileged": true,\
    "NetworkMode": "host"\
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
LABEL readme="https://raw.githubusercontent.com/raultrombin/blueos-ping-viewer-next/master/README.md"
LABEL type="education"
LABEL tags='[\
  "rov",\
  "robot"\
]'

ENTRYPOINT ["/entrypoint.sh"]