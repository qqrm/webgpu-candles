import os
import urllib.request
import urllib.parse
import smtplib
from email.message import EmailMessage


def send_telegram(message: str) -> bool:
    token = os.getenv("TELEGRAM_TOKEN")
    chat_id = os.getenv("TELEGRAM_CHAT_ID")
    if not token or not chat_id:
        return False
    data = urllib.parse.urlencode({"chat_id": chat_id, "text": message}).encode()
    url = f"https://api.telegram.org/bot{token}/sendMessage"
    try:
        with urllib.request.urlopen(url, data=data):
            pass
        return True
    except Exception:
        return False


def send_email(message: str) -> bool:
    server = os.getenv("SMTP_SERVER")
    port = int(os.getenv("SMTP_PORT", "587"))
    username = os.getenv("SMTP_USERNAME")
    password = os.getenv("SMTP_PASSWORD")
    sender = os.getenv("FROM_EMAIL")
    recipient = os.getenv("TO_EMAIL")
    if not all([server, username, password, sender, recipient]):
        return False
    msg = EmailMessage()
    msg["Subject"] = "Pipeline Notification"
    msg["From"] = sender
    msg["To"] = recipient
    msg.set_content(message)
    try:
        with smtplib.SMTP(server, port) as s:
            s.starttls()
            s.login(username, password)
            s.send_message(msg)
        return True
    except Exception:
        return False


def main() -> None:
    message = os.getenv("NOTIFY_MESSAGE", "Pipeline notification")
    if not send_telegram(message):
        send_email(message)


if __name__ == "__main__":
    main()
