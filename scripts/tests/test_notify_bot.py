import io
import os
import unittest
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


if __name__ == "__main__":
    unittest.main()
