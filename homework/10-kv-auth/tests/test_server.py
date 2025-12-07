import json
import logging
import os
import socket
import time
import urllib
import uuid

import jwt
import pytest
import requests

LOGGER = logging.getLogger(__name__)


def wait_for_socket(host, port):
    retries = 10
    exception = None
    while retries > 0:
        try:
            socket.socket().connect((host, port))
            return
        except ConnectionRefusedError as e:
            exception = e
            print(f'Got ConnectionError for url {host}:{port}: {e} , retrying')
            retries -= 1
            time.sleep(2)
    raise exception


@pytest.fixture
def auth_addr():
    addr = os.environ.get('AUTH_SERVER_URL', 'http://127.0.0.1:8090')
    host = urllib.parse.urlparse(addr).hostname
    port = urllib.parse.urlparse(addr).port
    wait_for_socket(host, port)
    yield addr


@pytest.fixture
def kv_addr():
    addr = os.environ.get('KV_SERVER_URL', 'http://127.0.0.1:8091')
    host = urllib.parse.urlparse(addr).hostname
    port = urllib.parse.urlparse(addr).port
    wait_for_socket(host, port)
    yield addr


@pytest.fixture
def jwt_private():
    path = os.environ.get('JWT_PRIVATE_KEY_FILE', 'secret')
    with open(path, 'rb') as file:
        key = file.read()
    yield key


@pytest.fixture
def jwt_public():
    path = os.environ.get('JWT_PUBLIC_KEY_FILE', 'secret')
    with open(path, 'rb') as file:
        key = file.read()
    yield key


def make_requests(method, addr, handle, params=None, data=None, token=None):
    if data is not None:
        data = json.dumps(data)
    req = requests.Request(
        method,
        addr + handle,
        params=params,
        data=data)
    if token is not None:
        req.headers['Authorization'] = f'Bearer {token}'
    prepared = req.prepare()
    LOGGER.info(f'>>> {prepared.method} {prepared.url}')
    if len(req.headers) > 0:
        LOGGER.info(f'>>> {req.headers}')
    if len(req.data) > 0:
        LOGGER.info(f'>>> {req.data}')
    s = requests.Session()
    resp = s.send(prepared)
    LOGGER.info(f'<<< {resp.status_code}')
    if len(resp.content) > 0:
        LOGGER.info(f'<<< {resp.content}')
    return resp


def make_user(auth_addr):
    username = str(uuid.uuid4())
    password = str(uuid.uuid4())
    r = make_requests(
        'POST',
        auth_addr,
        '/signup',
        data={
            'username': username,
            'password': password
        })
    assert r.status_code == 200
    token = r.text
    return ((username, password), token)


@pytest.fixture
def user(auth_addr):
    yield make_user(auth_addr)


@pytest.fixture
def another_user(auth_addr):
    yield make_user(auth_addr)


def generate_jwt(private, username):
    return jwt.encode({'username': username}, private, 'RS256')


def parse_jwt(token, public):
    return jwt.decode(token, public, ['RS256'])


def generate_hs256_jwt(secret, username):
    return jwt.encode({'username': username}, secret, 'HS256')


class TestRSA:
    @staticmethod
    def test_private(jwt_private):
        generate_jwt(jwt_private, 'test')

    @staticmethod
    def test_public(jwt_private, jwt_public):
        token = generate_jwt(jwt_private, 'test')
        decoded = parse_jwt(token, jwt_public)
        assert decoded['username'] == 'test'


class TestAuth:
    @staticmethod
    def check_jwt(token, public, username):
        decoded = parse_jwt(token, public)
        assert decoded['username'] == username

    @staticmethod
    def test_signup(jwt_public, user):
        ((username, _), token) = user
        TestAuth.check_jwt(token, jwt_public, username)

    @staticmethod
    def test_signup_with_existing_user(auth_addr, user):
        ((username, _), _) = user
        password = str(uuid.uuid4())
        r = make_requests(
            'POST',
            auth_addr,
            '/signup',
            data={
                'username': username,
                'password': password
            })
        assert r.status_code == 409

    @staticmethod
    def test_login(auth_addr, jwt_public, user):
        ((username, password), _) = user
        r = make_requests(
            'POST',
            auth_addr,
            '/login',
            data={
                'username': username,
                'password': password
            })
        assert r.status_code == 200
        TestAuth.check_jwt(r.text, jwt_public, username)

    @staticmethod
    def test_login_with_wrong_password(auth_addr, user):
        ((username, _), _) = user
        password = str(uuid.uuid4())
        r = make_requests(
            'POST',
            auth_addr,
            '/login',
            data={
                'username': username,
                'password': password
            })
        assert r.status_code == 404

    @staticmethod
    def test_login_with_non_existing_user(auth_addr):
        username = str(uuid.uuid4())
        password = str(uuid.uuid4())
        r = make_requests(
            'POST',
            auth_addr,
            '/login',
            data={
                'username': username,
                'password': password
            })
        assert r.status_code == 404

    @staticmethod
    def test_whoami(auth_addr, user):
        ((username, _), token) = user
        r = make_requests('GET', auth_addr, '/whoami', token=token)
        assert r.status_code == 200
        assert r.content == f'Hello, {username}'.encode()

    @staticmethod
    def test_whoami_without_token(auth_addr):
        r = make_requests('GET', auth_addr, '/whoami')
        assert r.status_code == 400

    @staticmethod
    def test_whoami_with_wrong_token(auth_addr):
        r = make_requests(
            'GET',
            auth_addr,
            '/whoami',
            token='not jwt')
        assert r.status_code == 401

    @staticmethod
    def test_whoami_with_token_of_non_existing_user(auth_addr, jwt_private):
        token = generate_jwt(jwt_private, 'Bob')
        r = make_requests('GET', auth_addr, '/whoami', token=token)
        assert r.status_code == 401

    @staticmethod
    def test_whoami_with_token_signed_by_other_secret(auth_addr):
        token = generate_hs256_jwt('wrong secret', 'Alice')
        r = make_requests('GET', auth_addr, '/whoami', token=token)
        assert r.status_code == 401


@pytest.fixture
def kv(kv_addr, user):
    key = str(uuid.uuid4())
    value = str(uuid.uuid4())
    (_, token) = user
    r = make_requests(
        'POST',
        kv_addr,
        '/put',
        params={'key': key},
        data={'value': value},
        token=token)
    assert r.status_code == 200
    yield ((key, value), user)


class TestKV:
    @staticmethod
    def test_put(kv):
        _ = kv

    @staticmethod
    def test_put_existing_key_with_the_same_user(kv_addr, kv):
        ((key, _), (_, token)) = kv
        value = str(uuid.uuid4())
        r = make_requests(
            'POST',
            kv_addr,
            '/put',
            params={'key': key},
            data={'value': value},
            token=token)
        assert r.status_code == 200

    @staticmethod
    def test_put_existing_key_with_another_user(kv_addr, kv, another_user):
        ((key, _), _) = kv
        value = str(uuid.uuid4())
        (_, token) = another_user
        r = make_requests(
            'POST',
            kv_addr,
            '/put',
            params={'key': key},
            data={'value': value},
            token=token)
        assert r.status_code == 403

    @staticmethod
    def test_put_with_token_signed_by_other_secret(kv_addr, kv):
        ((key, _), _) = kv
        value = str(uuid.uuid4())
        token = generate_hs256_jwt('wrong secret', 'Alice')
        r = make_requests(
            'POST',
            kv_addr,
            '/put',
            params={'key': key},
            data={'value': value},
            token=token)
        assert r.status_code == 401

    @staticmethod
    def test_put_with_wrong_token(kv_addr, kv):
        ((key, _), _) = kv
        value = str(uuid.uuid4())
        token = 'not jwt'
        r = make_requests(
            'POST',
            kv_addr,
            '/put',
            params={'key': key},
            data={'value': value},
            token=token)
        assert r.status_code == 401

    @staticmethod
    def test_put_without_token(kv_addr, kv):
        ((key, _), _) = kv
        value = str(uuid.uuid4())
        r = make_requests(
            'POST',
            kv_addr,
            '/put',
            params={'key': key},
            data={'value': value})
        assert r.status_code == 400

    @staticmethod
    def test_get(kv_addr, kv):
        ((key, value), (_, token)) = kv
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key},
            token=token)
        assert r.status_code == 200
        read_value = json.loads(r.content)['value']
        assert read_value == value

    @staticmethod
    def test_double_set_and_get(kv_addr, kv):
        ((key, _), (_, token)) = kv
        value = str(uuid.uuid4())
        make_requests(
            'POST',
            kv_addr,
            '/put',
            params={'key': key},
            data={'value': value},
            token=token)
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key},
            token=token)
        assert r.status_code == 200
        read_value = json.loads(r.content)['value']
        assert read_value == value

    @staticmethod
    def test_get_with_non_existing_key_and_correct_token(kv_addr, user):
        (_, token) = user
        key = str(uuid.uuid4())
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key},
            token=token)
        assert r.status_code == 404

    @staticmethod
    def test_get_with_another_user(kv_addr, kv, another_user):
        ((key, _), _) = kv
        (_, token) = another_user
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key},
            token=token)
        assert r.status_code == 403

    @staticmethod
    def test_get_with_token_signed_by_other_secret(kv_addr, kv):
        ((key, _), _) = kv
        token = generate_hs256_jwt('wrong secret', 'Alice')
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key},
            token=token)
        assert r.status_code == 401

    @staticmethod
    def test_get_with_wrong_token_and_existing_key(kv_addr, kv):
        ((key, _), _) = kv
        token = 'not jwt'
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key},
            token=token)
        assert r.status_code == 401

    @staticmethod
    def test_get_with_wrong_token_and_non_existing_key(kv_addr):
        key = str(uuid.uuid4())
        token = 'not jwt'
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key},
            token=token)
        assert r.status_code == 401

    @staticmethod
    def test_get_without_token_and_existing_key(kv_addr, kv):
        ((key, _), _) = kv
        r = make_requests(
            'GET',
            kv_addr,
            '/get',
            params={'key': key})
        assert r.status_code == 400
