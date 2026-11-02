"""Version tags select a release stage; the application keeps its numeric version."""
from dataclasses import dataclass
import re


@dataclass(frozen=True)
class ReleaseTag:
    name: str
    version: str
    candidate: int | None

    @property
    def prerelease(self):
        return self.candidate is not None


def parse_tag(name):
    number = r"(?:0|[1-9][0-9]*)"
    match = re.fullmatch(rf"v({number}\.{number}\.{number})(?:-rc\.([1-9][0-9]*))?", name)
    if not match:
        raise ValueError("Use vX.Y.Z or vX.Y.Z-rc.N (N starts at 1)")
    return ReleaseTag(name, match[1], int(match[2]) if match[2] else None)


def matching_candidate(stable, candidates, selected=""):
    release = parse_tag(stable)
    if release.prerelease:
        raise ValueError("Promotion needs a stable tag")
    eligible = [name for name in candidates if parse_tag(name).prerelease
                and parse_tag(name).version == release.version]
    if selected:
        if selected not in eligible:
            raise ValueError("Selected candidate must be published at this exact source and version")
        return selected
    if len(eligible) != 1:
        raise ValueError("Need exactly one published candidate at this source; retry with candidate_tag to disambiguate")
    return eligible[0]
