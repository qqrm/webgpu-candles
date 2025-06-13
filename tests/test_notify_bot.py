import os
import unittest
from unittest.mock import patch, MagicMock

import scripts.notify_bot as notify_bot
import urllib.parse


class TestNotifyBot(unittest.TestCase):
    @patch('urllib.request.urlopen')
    def test_send_telegram_success(self, mock_urlopen):
        mock_response = MagicMock(status=200)
        mock_urlopen.return_value = mock_response
        with patch.dict(os.environ, {'TELEGRAM_TOKEN': 't', 'TELEGRAM_CHAT_ID': '1'}, clear=True):
            notify_bot.send_telegram('hello')
            expected_data = urllib.parse.urlencode({'chat_id': '1', 'text': 'hello'}).encode()
            mock_urlopen.assert_called_once_with('https://api.telegram.org/bott/sendMessage', data=expected_data)

    def test_send_telegram_missing_env(self):
        with patch.dict(os.environ, {}, clear=True):
            with self.assertRaises(KeyError):
                notify_bot.send_telegram('msg')

    @patch('urllib.request.urlopen', side_effect=Exception('err'))
    def test_send_telegram_error(self, mock_urlopen):
        with patch.dict(os.environ, {'TELEGRAM_TOKEN': 't', 'TELEGRAM_CHAT_ID': '1'}, clear=True):
            with patch('builtins.print') as mock_print:
                notify_bot.send_telegram('boom')
                mock_urlopen.assert_called_once()
                mock_print.assert_called()


if __name__ == '__main__':
    unittest.main()
