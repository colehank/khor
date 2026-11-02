"""Read the exact App Store Connect build before upload retries or promotion."""
import base64
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
from urllib.parse import urlencode
from urllib.request import Request, urlopen


def encoded(value):
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode()


def raw_signature(der):
    if len(der) < 8 or der[0] != 0x30 or der[1] != len(der) - 2:
        raise ValueError("Invalid ES256 signature sequence")
    offset, values = 2, []
    for _ in range(2):
        if offset + 2 > len(der) or der[offset] != 2:
            raise ValueError("Invalid ES256 signature integer")
        size = der[offset + 1]
        integer = der[offset + 2:offset + 2 + size]
        if len(integer) != size or not integer or integer[0] & 0x80:
            raise ValueError("Invalid ES256 signature integer")
        value = integer.lstrip(b"\0")
        if len(value) > 32:
            raise ValueError("Invalid ES256 signature width")
        values.append(value.rjust(32, b"\0"))
        offset += size + 2
    if offset != len(der):
        raise ValueError("Invalid ES256 signature length")
    return b"".join(values)


class AppleClient:
    def __enter__(self):
        missing = [name for name in ("APPLE_API_KEY", "APPLE_API_ISSUER", "APPLE_API_KEY_CONTENT")
                   if not os.environ.get(name, "").strip()]
        if missing:
            raise ValueError("Missing App Store Connect credentials: " + ", ".join(missing))
        self.temporary = tempfile.TemporaryDirectory()
        self.key = Path(self.temporary.name) / "AuthKey.p8"
        self.key.write_text(os.environ["APPLE_API_KEY_CONTENT"])
        self.key.chmod(0o600)
        self.app = None
        return self

    def __exit__(self, *args):
        self.temporary.cleanup()

    def token(self):
        now = int(time.time())
        header = {"alg": "ES256", "kid": os.environ["APPLE_API_KEY"], "typ": "JWT"}
        payload = {"iss": os.environ["APPLE_API_ISSUER"], "iat": now, "exp": now + 600, "aud": "appstoreconnect-v1"}
        message = ".".join(encoded(json.dumps(part).encode()) for part in [header, payload])
        signed = subprocess.run(["openssl", "dgst", "-sha256", "-sign", str(self.key)],
                                input=message.encode(), capture_output=True)
        if signed.returncode:
            raise RuntimeError("Cannot sign App Store Connect request")
        return message + "." + encoded(raw_signature(signed.stdout))

    def get(self, path, **query):
        url = "https://api.appstoreconnect.apple.com/v1/" + path
        if query:
            url += "?" + urlencode(query)
        request = Request(url, headers={"Authorization": "Bearer " + self.token()})
        with urlopen(request, timeout=45) as response:
            return json.load(response)

    def app_id(self):
        if self.app is None:
            matches = self.get("apps", **{"filter[bundleId]": "io.github.colehank.khor"})["data"]
            if len(matches) != 1:
                raise ValueError("Cannot identify the Khor App Store app")
            self.app = matches[0]["id"]
        return self.app

    def find_build(self, version, number):
        result = self.get("builds", **{"filter[app]": self.app_id(), "filter[version]": number,
                                      "include": "preReleaseVersion", "limit": 200})
        releases = {v["id"]: v["attributes"] for v in result.get("included", []) if v["type"] == "preReleaseVersions"}
        matches = []
        for build in result["data"]:
            release = releases.get(build["relationships"]["preReleaseVersion"]["data"]["id"])
            if release and release["version"] == version and release["platform"] == "IOS":
                matches.append(build)
        if len(matches) > 1:
            raise ValueError("Apple returned multiple builds for this exact version and build number")
        return matches[0] if matches else None

    def processed_build(self, version, number, timeout=900):
        deadline = time.monotonic() + timeout
        while True:
            build = self.find_build(version, number)
            if build:
                attributes = build["attributes"]
                state = attributes["processingState"]
                if attributes.get("expired") or state in {"FAILED", "INVALID"}:
                    raise ValueError(f"Apple rejected or expired build {version} ({number}): {state}")
                if state == "VALID":
                    return build
            if time.monotonic() >= deadline:
                raise TimeoutError("Apple is still processing; retry distribution with the same candidate")
            time.sleep(10)

    def already_submitted(self, version, build_id):
        versions = self.get(f"apps/{self.app_id()}/appStoreVersions",
                            **{"filter[versionString]": version, "filter[platform]": "IOS", "include": "build"})["data"]
        submitted = {"WAITING_FOR_REVIEW", "IN_REVIEW", "PENDING_DEVELOPER_RELEASE", "PENDING_APPLE_RELEASE",
                     "PROCESSING_FOR_APP_STORE", "READY_FOR_SALE", "READY_FOR_DISTRIBUTION"}
        for item in versions:
            if item["attributes"]["appStoreState"] in submitted:
                selected = item["relationships"]["build"]["data"]
                if not selected or selected["id"] != build_id:
                    raise ValueError("Another build is already submitted; refusing to replace it")
                return True
        return False
