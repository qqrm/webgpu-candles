import io
import os
import unittest
from unittest import mock
from contextlib import redirect_stdout

import scripts.notify_bot as notify_bot


class NotifyBotTests(unittest.TestCase):
    def test_missing_environment_variables(self):
        os.environ.pop("TELEGRAM_TOKEN", None)
        os.environ.pop("TELEGRAM_CHAT_ID", None)
        buf = io.StringIO()
        with self.assertRaises(SystemExit) as cm, redirect_stdout(buf):
            notify_bot.send_telegram("Test")
        self.assertEqual(cm.exception.code, 1)
        output = buf.getvalue()
        self.assertIn(
            "Missing TELEGRAM_TOKEN or TELEGRAM_CHAT_ID", output
        )

    def test_unsuccessful_response(self):
        os.environ["TELEGRAM_TOKEN"] = "token"
        os.environ["TELEGRAM_CHAT_ID"] = "chat"

        class FakeResponse:
            def __init__(self, status: int):
                self.status = status

            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

        def fake_urlopen(url, data=None):
            return FakeResponse(500)

        buf = io.StringIO()
        with mock.patch(
            "urllib.request.urlopen", side_effect=fake_urlopen
        ), self.assertRaises(SystemExit) as cm, redirect_stdout(buf):
            notify_bot.send_telegram("Test")

        self.assertEqual(cm.exception.code, 1)
        output = buf.getvalue()
        self.assertIn("Failed to send Telegram message: HTTP 500", output)


if __name__ == "__main__":
    unittest.main()
