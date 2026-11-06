"""Synthetic client for bridge-probe.mjs. Never connects to TypeSafe."""
import asyncio
import base64
import json
import os
import sys

from typesafe_sdk import (
    AsyncTypeSafeClient, Image, Noul, TypeSafeClient,
    TypeSafeAuthenticationError, TypeSafeRateLimitError,
)

base = sys.argv[1]
assert base.startswith("http://127.0.0.1:")
os.environ["TYPESAFE_BASE_URL"] = base
os.environ["TYPESAFE_API_KEY"] = "synthetic-probe-key"
questions = {"q": Noul(instructions="Is this a test?")}
png = base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a3ioAAAAASUVORK5CYII=")
passed = []

with TypeSafeClient() as client:
    for case, state, assets in [
        ("plain", "synthetic", None),
        ("compressed", "synthetic " * 1000, None),
        ("multipart", "Inspect <photo>", {"<photo>": Image(png)}),
        ("response-gzip", "synthetic", None),
        ("retry529", "synthetic", None),
    ]:
        answer = client.system_one(state, questions, assets=assets,
            extra_headers={"x-probe-case": "python-" + case,
                           "TypeSafe-Danger-Zone-Images": "true"})
        assert answer.answers["q"].noul == 0.75
        assert answer.request_id == "synthetic-request-id"
        passed.append(case)
    assert client.models.list(extra_headers={"x-probe-case": "python-models"}).models[0].name == "probe-model"
    passed.append("models")
    for case, expected in [("auth401", TypeSafeAuthenticationError), ("retry429", TypeSafeRateLimitError)]:
        try:
            client.system_one("synthetic", questions, extra_headers={"x-probe-case": "python-" + case})
        except expected:
            passed.append(case)
        else:
            raise AssertionError("Expected SDK error: " + case)

async def async_check():
    async with AsyncTypeSafeClient() as client:
        result = await client.system_one("synthetic", questions,
            extra_headers={"x-probe-case": "python-async"})
        assert result.answers["q"].noul == 0.75

asyncio.run(async_check())
passed.append("async")
print(json.dumps({"python": passed}))
