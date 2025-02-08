FROM rust:1.84-alpine AS builder

ARG PROJECT_DIR
ARG PROJECT_NAME

RUN apk add --no-cache musl-dev protoc protobuf-dev

WORKDIR /usr/src/app

COPY grpc_mem_bench.proto ..
COPY $PROJECT_DIR .

RUN cargo build --release

FROM alpine:3.21

ARG PROJECT_NAME

RUN apk add --no-cache libc6-compat

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/$PROJECT_NAME /app/grpc_mem_bench

CMD ["/app/grpc_mem_bench"]
