FROM golang:1.21-alpine

WORKDIR /app

COPY go.mod ./
COPY domain/ ./domain/

ENTRYPOINT ["go", "test", "-v", "./..."]
