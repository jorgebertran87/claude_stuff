FROM golang:1.21-alpine

WORKDIR /app

COPY go.mod ./
COPY domain/ ./domain/
COPY tests/ ./tests/

ENTRYPOINT ["go", "test", "-v", "./tests/..."]
