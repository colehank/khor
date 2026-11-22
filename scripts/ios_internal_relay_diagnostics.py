"""Sign and upload the pinned iOS relay diagnostic to the existing solo group."""
import json
import mmap
import os
from pathlib import Path
import sys
import time
from urllib.request import Request, urlopen

from apple_api import AppleClient
from apple_release import archive_ios, export_ios, package_ios
from release_manifest import source_sha, version

SOURCE = "a06b724d381f14879da04c9b97fe8b5c2ef3fa1b"
BUILD = "1046.1.1"
GROUP_NAME = "khor-test"
ROOT = Path("ios")


def all_resources(client, path):
    result = client.get(path, limit=200)
    if result.get("links", {}).get("next"):
        raise ValueError("Unexpected pagination while checking the testing audience")
    return result["data"]


def solo_audience(client):
    groups = all_resources(client, f"apps/{client.app_id()}/betaGroups")
    internal = [group for group in groups if group["attributes"]["isInternalGroup"]]
    if len(internal) != 1 or internal[0]["attributes"]["name"] != GROUP_NAME:
        raise ValueError("Expected exactly the verified Khor internal group")
    group_id = internal[0]["id"]
    testers = all_resources(client, f"betaGroups/{group_id}/relationships/betaTesters")
    if len(testers) != 1:
        raise ValueError("Expected exactly one internal tester")
    return group_id, testers[0]["id"]


def group_ids(client, build_id):
    return {group["id"] for group in all_resources(client, f"builds/{build_id}/relationships/betaGroups")}


def attach_group(client, build_id, group_id):
    url = f"https://api.appstoreconnect.apple.com/v1/builds/{build_id}/relationships/betaGroups"
    data = json.dumps({"data": [{"type": "betaGroups", "id": group_id}]}).encode()
    headers = {"Authorization": "Bearer " + client.token(), "Content-Type": "application/json"}
    with urlopen(Request(url, data=data, headers=headers), timeout=45):
        pass


def upload():
    metadata = json.loads((ROOT / "build.json").read_text())
    if metadata != {"version": version(), "build": BUILD, "signed": True}:
        raise ValueError("Unexpected signed internal archive metadata")
    with AppleClient() as client:
        audience = solo_audience(client)
        group_id, _ = audience
        if client.find_build(metadata["version"], BUILD) is None:
            export_ios(ROOT, internal_only=True)
        build = client.processed_build(metadata["version"], BUILD)
        if build["attributes"]["buildAudienceType"] != "INTERNAL_ONLY":
            raise ValueError("Apple did not mark this build Internal Only")
        build_id = build["id"]
        if solo_audience(client) != audience:
            raise ValueError("Internal tester changed while Apple was processing the build")
        groups = group_ids(client, build_id)
        if not groups:
            attach_group(client, build_id, group_id)
        for _ in range(12):
            if group_ids(client, build_id) == {group_id}:
                detail = client.get("buildBetaDetails", **{"filter[build]": build_id})["data"]
                if len(detail) == 1 and detail[0]["attributes"]["internalBuildState"] == "IN_BETA_TESTING":
                    if solo_audience(client) != audience:
                        raise ValueError("Internal tester changed after assigning the build")
                    print(f"TestFlight {metadata['version']} ({BUILD}) is VALID, INTERNAL_ONLY and available to the sole internal tester.")
                    return
            time.sleep(10)
    raise ValueError("Apple has not confirmed availability exclusively to the intended internal group")


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("GITHUB_REF") != "refs/heads/main":
        raise ValueError("Internal diagnostic distribution requires trusted main CI")
    if source_sha() != SOURCE:
        raise ValueError("Unexpected diagnostic source revision")
    if sys.argv[1:] == ["archive"]:
        archive_ios(True, number=BUILD, features=("net-diagnostics",))
        package_ios(number=BUILD)
        app = Path("apps/gui/src-tauri/gen/apple/build/khor-gui_iOS.xcarchive/Products/Applications/Khor.app/Khor")
        with app.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
            if binary.find(b"request-timing.jsonl") == -1:
                raise ValueError("The signed iOS archive lacks the request timing recorder")
    elif sys.argv[1:] == ["upload"]:
        upload()
    elif sys.argv[1:] == ["check-audience"]:
        with AppleClient() as client:
            solo_audience(client)
    else:
        raise ValueError("Expected check-audience, archive or upload")


if __name__ == "__main__":
    main()
