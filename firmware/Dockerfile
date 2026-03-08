FROM ubuntu:24.04

RUN apt-get update && apt-get install -y \
    curl \
    rustup \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN rustup default stable && \
    rustup target add thumbv6m-none-eabi

WORKDIR /build

COPY . .

RUN chmod +x build.sh

CMD ["./build.sh"]
