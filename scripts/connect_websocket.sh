#!/bin/bash

set -e

if ! type "websocat" > /dev/null 2>&1; then
  echo "websocat is not found. Please install websocat"
  exit 1
fi

if ! type "jq" > /dev/null 2>&1; then
  echo "jq is not found. Please install jq"
  exit 1
fi

if ! type "curl" > /dev/null 2>&1; then
  echo "curl is not found. Please install curl"
  exit 1
fi

WEBSOCKET_URL=$(curl -X POST -sL -H "AUTHORIZATION: Bearer $SLACK_BOT_SOCKET_MODE_TOKEN" 'https://slack.com/api/apps.connections.open' | jq -r .url)

echo "Connecting to $WEBSOCKET_URL"

websocat "$WEBSOCKET_URL"
