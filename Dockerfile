FROM rust:1.84.1-slim-bullseye AS builder

RUN update-ca-certificates

WORKDIR /its_mypic_image_server

COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    --mount=type=cache,target=/its_mypic_image_server/target/ \
    cargo build --release


FROM gcr.io/distroless/cc

WORKDIR /its_mypic_image_server

COPY --from=mwader/static-ffmpeg:7.1 /ffmpeg /usr/local/bin/

COPY --from=builder /its_mypic_image_server/target/release/its_mypic_image_server .

CMD ["/its_mypic_image_server/its_mypic_image_server"]
