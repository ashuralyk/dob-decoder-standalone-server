FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev
ENV OPENSSL_DIR=/usr
ENV RUSTFLAGS='-C target-feature=-crt-static'

WORKDIR /
RUN cargo new app

# Install dependencies for cache
WORKDIR /app
RUN --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
  --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
  cargo build --release

COPY . .
RUN cargo build --release

FROM alpine:3
RUN apk add --no-cache libgcc

COPY --link --from=builder /app/target/release/dob-decoder-server /app/dob-decoder-server

WORKDIR /app
RUN mkdir -p cache/decoders cache/dobs
EXPOSE 8090
CMD ["/app/dob-decoder-server"]