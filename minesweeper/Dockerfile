FROM python:3.13-slim

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
        libgl1 \
        libglib2.0-0 \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 5000

# Default: run the HTTP server. Override CMD to run tests or the CLI.
CMD ["gunicorn", "--bind", "0.0.0.0:5000", "--workers", "2", "server:app"]
