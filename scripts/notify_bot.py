#!/usr/bin/env python3
"""Deployment notification helper."""

import os
import sys
import urllib.parse
import urllib.request


def send_telegram(message: str) -> None:
    """Send a Telegram message using Bot API."""
    token = os.getenv("TELEGRAM_TOKEN")
    chat_id = os.getenv("TELEGRAM_CHAT_ID")
    if not token or not chat_id:
        print("Missing TELEGRAM_TOKEN or TELEGRAM_CHAT_ID environment variables.")
        sys.exit(1)
    data = urllib.parse.urlencode({"chat_id": chat_id, "text": message}).encode()
    url = f"https://api.telegram.org/bot{token}/sendMessage"
    try:
        with urllib.request.urlopen(url, data=data) as resp:
            body = resp.read().decode()
            if resp.status != 200:
                print(
                    f"Failed to send Telegram message: HTTP {resp.status} {body}"
                )
                sys.exit(1)
    except Exception as err:
        print(f"Failed to send Telegram message: {err}")
        sys.exit(1)


def main() -> None:
    message = os.getenv("NOTIFY_MESSAGE", "Pipeline notification")
    send_telegram(message)


if __name__ == "__main__":
    main()
