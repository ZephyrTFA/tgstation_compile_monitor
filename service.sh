#!/bin/bash
cd "$(dirname "$0")"
if [ -f "service.pid" ]; then
    kill -9 $(cat service.pid)
    rm service.pid
fi
git pull
cargo run --release > service.log 2>&1 &
echo $! > service.pid
fg
