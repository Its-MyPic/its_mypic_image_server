FROM rust:1-slim-buster AS builder

RUN update-ca-certificates

WORKDIR /its_mypic_image_server

COPY . .

RUN cargo build --release


FROM gcr.io/distroless/cc

WORKDIR /its_mypic_image_server

COPY --from=builder /its_mypic_image_server/target/release/its_mypic_image_server .

CMD ["/its_mypic_image_server/its_mypic_image_server"]
