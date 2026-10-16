FROM node:22-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends chromium python3 stress-ng iproute2 ca-certificates curl build-essential \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
ENV PATH=/root/.cargo/bin:$PATH

COPY chrome /usr/local/bin/chrome
COPY inside.sh /usr/local/bin/flake-inside
RUN chmod 755 /usr/local/bin/chrome /usr/local/bin/flake-inside
