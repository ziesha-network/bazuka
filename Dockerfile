FROM rust:1.60.0-buster

RUN apt update && apt install -y openssl cmake build-essential

ENV RUSTFLAGS="$RUSTFLAGS -L /usr/lib/"

RUN cargo new bazuka
WORKDIR /bazuka
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release --features node
RUN rm src/*.rs
RUN rm ./target/release/deps/bazuka*

COPY ./src ./src

RUN cargo build --release --features node

CMD ["./target/release/bazuka"]
