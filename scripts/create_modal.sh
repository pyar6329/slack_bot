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

REQUEST_BODY=$(cat << __EOF__
{
    "trigger_id": "8046964394625.120575625601.5ad0816e36e4f92d018405a994f6aae6",
    "view": {
        "type": "modal",
        "title": {
            "type": "plain_text",
            "text": "create a account"
        },
        "blocks": []
    }
}
__EOF__
)

# see: permission for socket mode
# Subscribe to bot events
# https://api.slack.com/apps/A04PHT9L7MH/event-subscriptions?
# ref: https://api.slack.com/methods/views.open
curl -sL -X POST \
  -H "Authorization: Bearer $SLACK_BOT_TOKEN" \
  -H "Content-Type: application/json" \
  -d "${REQUEST_BODY}" \
  https://slack.com/api/views.open \
  | jq -r .

