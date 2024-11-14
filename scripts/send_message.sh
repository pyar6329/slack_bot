#!/bin/bash

set -e

if ! type "jq" > /dev/null 2>&1; then
  echo "jq is not found. Please install jq"
  exit 1
fi

if ! type "curl" > /dev/null 2>&1; then
  echo "curl is not found. Please install curl"
  exit 1
fi

# see: permission for socket mode
# Subscribe to bot events
# https://api.slack.com/apps/A04PHT9L7MH/event-subscriptions?
curl -sL -X POST \
  -H "Authorization: Bearer $SLACK_BOT_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"channel\": \"$SLACK_BOT_CHANNEL_ID\", \"text\": \"Hello from websocat!\"}" \
  https://slack.com/api/chat.postMessage \
  | jq -r .

