FROM rust:1.58-alpine

RUN apk add openssl-dev cmake build-base

ENV RUSTFLAGS="$RUSTFLAGS -L /usr/lib/"

RUN cargo new bazuka
WORKDIR /bazuka
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs
RUN rm ./target/release/deps/bazuka*

COPY ./src ./src

RUN cargo build --release --features node

CMD ["./target/release/bazuka"]
