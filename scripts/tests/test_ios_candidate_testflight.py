import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import ios_candidate_testflight as ios
from release_manifest import files

SOURCE = "1" * 40


class IOSCandidateTests(unittest.TestCase):
    def quality(self):
        run = dict(head_sha=SOURCE, head_branch="main", status="completed", conclusion="success",
                   path=".github/workflows/quality.yml")
        jobs = [dict(name=n, head_sha=SOURCE, status="completed", conclusion="success") for n in ios.QUALITY_JOBS]
        return run, jobs

    def test_requires_every_shared_gate_without_skipping(self):
        run, jobs = self.quality()
        ios.require_quality(run, jobs, SOURCE)
        for field, value in [("conclusion", "skipped"), ("status", "in_progress"), ("head_sha", "2" * 40)]:
            bad = copy.deepcopy(jobs)
            bad[0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                ios.require_quality(run, bad, SOURCE)
        with self.assertRaises(ValueError):
            ios.require_quality(run, jobs[:-1], SOURCE)

    def test_quality_from_another_source_or_workflow_is_rejected(self):
        run, jobs = self.quality()
        for field, value in [("head_sha", "2" * 40), ("head_branch", "feature"), ("path", "other.yml")]:
            bad = {**run, field: value}
            with self.subTest(field=field), self.assertRaises(ValueError):
                ios.require_quality(bad, jobs, SOURCE)

    def test_original_conditional_dsh_skip_keeps_all_shared_gates_required(self):
        run, jobs = self.quality()
        optional = dict(name="dsh-integration", head_sha=SOURCE,
                        status="completed", conclusion="skipped")
        ios.require_quality(run, jobs + [optional], SOURCE)
        for field, value in [("conclusion", "failure"), ("status", "in_progress"),
                             ("head_sha", "2" * 40), ("conclusion", "success")]:
            bad = {**optional, field: value}
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                ios.require_quality(run, jobs + [bad], SOURCE)

    def test_conditional_skip_does_not_admit_unknown_or_duplicate_jobs(self):
        run, jobs = self.quality()
        optional = dict(name="dsh-integration", head_sha=SOURCE,
                        status="completed", conclusion="skipped")
        for extra in [[optional, optional], [{**optional, "name": "unknown"}],
                      [optional, jobs[0]]]:
            with self.subTest(extra=extra), self.assertRaises(ValueError):
                ios.require_quality(run, jobs + extra, SOURCE)

    def test_conditional_skip_cannot_replace_a_missing_or_failed_shared_gate(self):
        run, jobs = self.quality()
        optional = dict(name="dsh-integration", head_sha=SOURCE,
                        status="completed", conclusion="skipped")
        with self.assertRaises(ValueError):
            ios.require_quality(run, jobs[:-1] + [optional], SOURCE)
        for conclusion in ["failure", "skipped", "cancelled"]:
            bad = copy.deepcopy(jobs)
            bad[0]["conclusion"] = conclusion
            with self.subTest(conclusion=conclusion), self.assertRaises(ValueError):
                ios.require_quality(run, bad + [optional], SOURCE)

    def test_ios_must_pass_while_desktop_may_still_be_running(self):
        run = dict(head_sha=SOURCE, head_branch="v0.2.0-rc.19", path=".github/workflows/release.yml")
        jobs = [dict(name="ios", head_sha=SOURCE, status="completed", conclusion="success"),
                dict(name="desktop", status="in_progress", conclusion=None)]
        _, quality = self.quality()
        jobs += [{**j, "name": "quality / " + j["name"]} for j in quality if j["name"] != "frontend / build"]
        jobs += [j for j in quality if j["name"] == "frontend / build"]
        ios.require_ios_producer(run, jobs, SOURCE, "v0.2.0-rc.19")
        jobs[0]["conclusion"] = "failure"
        with self.assertRaises(ValueError):
            ios.require_ios_producer(run, jobs, SOURCE, "v0.2.0-rc.19")

    def test_an_old_main_quality_pass_does_not_override_a_failed_producer_gate(self):
        run = dict(head_sha=SOURCE, head_branch="v0.2.0-rc.19", path=".github/workflows/release.yml")
        _, quality = self.quality()
        jobs = [{**j, "name": "quality / " + j["name"]} for j in quality if j["name"] != "frontend / build"]
        jobs += [j for j in quality if j["name"] == "frontend / build"]
        jobs += [dict(name="ios", head_sha=SOURCE, status="completed", conclusion="success")]
        next(j for j in jobs if j["name"] == "quality / mac-updates")["conclusion"] = "failure"
        with self.assertRaises(ValueError):
            ios.require_ios_producer(run, jobs, SOURCE, "v0.2.0-rc.19")

    def archive(self, root, **changes):
        (root / "Khor.xcarchive.tar.gz").write_bytes(b"opaque signed archive fixture")
        (root / "build.json").write_text(json.dumps(dict(version="0.2.0", build="1037.1", signed=True, **changes)))
        receipt = dict(schema=1, kind="ios-archive", source_sha=SOURCE, version="0.2.0", run_id="37", files=files(root))
        (root / "receipt.json").write_text(json.dumps(receipt))

    def test_exact_archive_is_accepted_and_modified_payload_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.archive(root)
            ios.check_archive(root, SOURCE, 37, "0.2.0", "1037.1")
            (root / "Khor.xcarchive.tar.gz").write_bytes(b"replaced")
            with self.assertRaises(ValueError):
                ios.check_archive(root, SOURCE, 37, "0.2.0", "1037.1")

    def test_wrong_producer_source_version_or_build_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.archive(root)
            for source, run, version, build in [("2" * 40, 37, "0.2.0", "1037.1"),
                                                (SOURCE, 38, "0.2.0", "1037.1"),
                                                (SOURCE, 37, "0.3.0", "1037.1"),
                                                (SOURCE, 37, "0.2.0", "1036.1")]:
                with self.subTest(source=source, run=run, version=version, build=build), self.assertRaises(ValueError):
                    ios.check_archive(root, source, run, version, build)

    def test_unsigned_archive_is_rejected_even_with_matching_hashes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            self.archive(root)
            (root / "build.json").write_text(json.dumps(dict(version="0.2.0", build="1037.1", signed=False)))
            receipt = json.loads((root / "receipt.json").read_text())
            receipt["files"] = files(root)
            (root / "receipt.json").write_text(json.dumps(receipt))
            with self.assertRaises(ValueError):
                ios.check_archive(root, SOURCE, 37, "0.2.0", "1037.1")


if __name__ == "__main__":
    unittest.main()
