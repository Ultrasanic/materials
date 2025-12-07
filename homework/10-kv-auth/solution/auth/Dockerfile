FROM golang:1.23-alpine AS builder

WORKDIR /auth
COPY . .
RUN go mod download && go build .

FROM alpine:latest
WORKDIR /auth
COPY --from=builder /auth .
ENTRYPOINT ["./auth"]
