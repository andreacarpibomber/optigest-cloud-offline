"""Ed25519 (RFC 8032) in Python puro, senza dipendenze. Compatibile con libsodium (PHP sodium_crypto_sign_*).

Usato solo dagli script offline gen_license_keys.py e sign_host_key.py (generazione chiavi e firma dei certificati):
non è pensato per la firma ad alto volume. Implementazione di riferimento della RFC, non resistente ai timing attack.
"""

import hashlib
import os

P = 2 ** 255 - 19
Q = 2 ** 252 + 27742317777372353535851937790883648493
D = -121665 * pow(121666, P - 2, P) % P
SQRT_M1 = pow(2, (P - 1) // 4, P)


def _sha512(data):
	return hashlib.sha512(data).digest()


def _sha512_modq(data):
	return int.from_bytes(_sha512(data), "little") % Q


def _inv(x):
	return pow(x, P - 2, P)


def _add(a, b):
	aa = (a[1] - a[0]) * (b[1] - b[0]) % P
	bb = (a[1] + a[0]) * (b[1] + b[0]) % P
	cc = 2 * a[3] * b[3] * D % P
	dd = 2 * a[2] * b[2] % P
	e, f, g, h = bb - aa, dd - cc, dd + cc, bb + aa
	return (e * f % P, g * h % P, f * g % P, e * h % P)


def _mul(scalar, point):
	result = (0, 1, 1, 0)
	while scalar > 0:
		if scalar & 1:
			result = _add(result, point)
		point = _add(point, point)
		scalar >>= 1
	return result


def _compress(point):
	zinv = _inv(point[2])
	x = point[0] * zinv % P
	y = point[1] * zinv % P
	return int.to_bytes(y | ((x & 1) << 255), 32, "little")


def _recover_x(y, sign):
	if y >= P:
		return None
	x2 = (y * y - 1) * _inv(D * y * y + 1) % P
	if x2 == 0:
		return None if sign else 0
	x = pow(x2, (P + 3) // 8, P)
	if (x * x - x2) % P != 0:
		x = x * SQRT_M1 % P
	if (x * x - x2) % P != 0:
		return None
	if (x & 1) != sign:
		x = P - x
	return x


def _decompress(data):
	if len(data) != 32:
		return None
	y = int.from_bytes(data, "little")
	sign = y >> 255
	y &= (1 << 255) - 1
	x = _recover_x(y, sign)
	if x is None:
		return None
	return (x, y, 1, x * y % P)


_GY = 4 * _inv(5) % P
_GX = _recover_x(_GY, 0)
BASE = (_GX, _GY, 1, _GX * _GY % P)


def _expand(seed):
	h = _sha512(seed)
	a = int.from_bytes(h[:32], "little")
	a &= (1 << 254) - 8
	a |= 1 << 254
	return a, h[32:]


def generate_keypair():
	"""Ritorna (chiave_privata_64_byte, chiave_pubblica_32_byte). La privata è seed+pubblica, come in libsodium."""
	seed = os.urandom(32)
	public = _compress(_mul(_expand(seed)[0], BASE))
	return seed + public, public


def sign(secret_key, message):
	"""Firma detached (64 byte). secret_key: 64 byte (formato libsodium) o 32 byte (solo seed)."""
	seed = secret_key[:32]
	a, prefix = _expand(seed)
	public = _compress(_mul(a, BASE))
	r = _sha512_modq(prefix + message)
	r_point = _compress(_mul(r, BASE))
	h = _sha512_modq(r_point + public + message)
	s = (r + h * a) % Q
	return r_point + int.to_bytes(s, 32, "little")


def verify(public_key, message, signature):
	if len(public_key) != 32 or len(signature) != 64:
		return False
	a_point = _decompress(public_key)
	r_point = _decompress(signature[:32])
	if a_point is None or r_point is None:
		return False
	s = int.from_bytes(signature[32:], "little")
	if s >= Q:
		return False
	h = _sha512_modq(signature[:32] + public_key + message)
	left = _mul(s, BASE)
	right = _add(r_point, _mul(h, a_point))
	if (left[0] * right[2] - right[0] * left[2]) % P != 0:
		return False
	return (left[1] * right[2] - right[1] * left[2]) % P == 0
