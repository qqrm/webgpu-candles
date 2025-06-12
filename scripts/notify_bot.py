#!/usr/bin/env python3
"""Deployment notification helper."""

import os
import urllib.parse
import urllib.request


def send_telegram(message: str) -> None:
    """Send a Telegram message using Bot API."""
    token = os.environ["TELEGRAM_TOKEN"]
    chat_id = os.environ["TELEGRAM_CHAT_ID"]
    data = urllib.parse.urlencode({"chat_id": chat_id, "text": message}).encode()
    url = f"https://api.telegram.org/bot{token}/sendMessage"
    try:
        urllib.request.urlopen(url, data=data)
    except Exception as err:
        print(f"Failed to send Telegram message: {err}")


def main() -> None:
    message = os.getenv("NOTIFY_MESSAGE", "Pipeline notification")
    send_telegram(message)


if __name__ == "__main__":
    main()
