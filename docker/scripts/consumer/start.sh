#!/bin/sh

while true; do
    ./rabbitmq-consumer | awk '{ print strftime("%Y-%m-%d %H:%M:%S"), $0; fflush(); }' >> logs/rabbitmq-consumer.log

    echo "Process crashed with code $?. Restarting..." >&2

    sleep 1
done
