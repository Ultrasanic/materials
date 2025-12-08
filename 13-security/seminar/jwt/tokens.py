# https://pypi.org/project/

import jwt
from datetime import datetime, timedelta, timezone

SECRET = 'secret-key-4214821948129'

message = {
    'iss': 'https://example.com/',
    'sub': 'lodthe-hse',
    'iat': datetime.now(timezone.utc).timestamp(),
    'exp': (datetime.now(timezone.utc) + timedelta(hours=1)).timestamp(),
}

token = jwt.encode(message, SECRET, algorithm='HS256')

print('Token:', token)

message_received = jwt.decode(token, SECRET, do_time_check=True, algorithms=['HS256'])

print('Decoded:', message_received)

assert message == message_received