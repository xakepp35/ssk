# Build the existing binary first: cargo build --release -p ssk
FROM ubuntu:26.04
RUN apt-get update -qq && apt-get install -y --no-install-recommends ca-certificates libasound2t64 libx11-6 libxi6 libgl1 iptables && rm -rf /var/lib/apt/lists/*
COPY target/release/ssk /usr/local/bin/ssk
COPY tools/nat /opt/ssk
ENTRYPOINT ["/bin/sh", "/opt/ssk/start.sh"]
