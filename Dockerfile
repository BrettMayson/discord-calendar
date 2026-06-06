FROM debian:trixie-slim

COPY ./discord-calendar /app/calendar
WORKDIR /app

RUN apt-get update && apt-get install libssl-dev ca-certificates -y && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/app/calendar"]
