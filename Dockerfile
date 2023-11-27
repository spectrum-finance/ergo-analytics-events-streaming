FROM rust:1.73.0
WORKDIR /usr/src/events-streaming
COPY . .
RUN apt-get update && apt-get install make clang pkg-config libssl-dev glibc-source gcc libstdc++6 -y
RUN cargo install --path .

CMD ["events-streaming", "--config-yaml-path=/etc/config.yml"]