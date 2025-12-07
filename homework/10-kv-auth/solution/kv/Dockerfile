FROM golang:1.23-alpine AS builder

WORKDIR /kv
COPY . .
RUN go mod download && go build .

FROM alpine:latest
WORKDIR /kv
COPY --from=builder /kv .
ENTRYPOINT ["./kv"]
