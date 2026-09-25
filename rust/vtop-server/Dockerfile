# Build context: the repository root.
#   docker build -f rust/vtop-server/Dockerfile -t vtop-server .
FROM rust:1-bookworm AS build
WORKDIR /src
COPY rust/ .
RUN cargo build --locked --profile server -p vtop-server \
 && cp target/server/vtop-server /vtop-server

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build /vtop-server /usr/local/bin/vtop-server
ENV RUST_LOG=info
EXPOSE 8080
USER nonroot
ENTRYPOINT ["/usr/local/bin/vtop-server"]
