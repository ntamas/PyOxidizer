#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Emit default_python_distributions.rs boilerplate for python-build-standalone release."""

import argparse
import hashlib
import urllib.error
import urllib.request
import re
import socket
import sys
import time

from contextlib import contextmanager
from github import Github
from tqdm import tqdm


ENTRY = """
PythonDistributionRecord {{
    python_major_minor_version: "{major_minor}".to_string(),
    location: PythonDistributionLocation::Url {{
        url: "{url}".to_string(),
        sha256: "{sha256}".to_string(),
    }},
    target_triple: "{target_triple}".to_string(),
    supports_prebuilt_extension_modules: {supports_prebuilt_extension_modules},
}},
""".strip()

RE_CAL_VER = re.compile(r"\d{8}")


@contextmanager
def progress(message):
    print(message, end = " ", file = sys.stderr, flush=True)
    yield
    print("done.", file=sys.stderr)


def download_and_hash(url, *, title="Downloading..."):
    with (
        urllib.request.urlopen(url) as r,
        tqdm(unit='B', desc=title, unit_scale=True, unit_divisor=1024, miniters=1) as t
    ):
        length = r.getheader("content-length")
        if length:
            t.total = int(length)

        h = hashlib.sha256()

        while True:
            chunk = r.read(32768)
            if not chunk:
                break

            h.update(chunk)
            t.update(len(chunk))

        return h.hexdigest()


def download_and_hash_with_retries(*args, **kwds):
    for i in range(5):
        try:
            return download_and_hash(*args, **kwds)
        except TimeoutError:
            pass
        except urllib.error.HTTPError:
            pass
        time.sleep(0.5)
    else:
        return download_and_hash(*args, **kwds)


def format_record(record):
    title = f"Downloading {record['name']}..."
    record["sha256"] = download_and_hash_with_retries(record["url"], title=title)
    return ENTRY.format(**record)


def get_latest_tag(repo):
    """Get the most recent CalVer tag, corresponding to latest."""
    with progress("Finding latest tag..."):
        cal_ver_tags = [tag.name for tag in repo.get_tags() if RE_CAL_VER.match(tag.name)]
    return max(cal_ver_tags)


def main():
    socket.setdefaulttimeout(60)

    parser = argparse.ArgumentParser()
    parser.add_argument("--api-token", help="GitHub API token", required=True)
    parser.add_argument(
        "--tag", help="python-build-standalone release tag, default is latest", default=None
    )

    args = parser.parse_args()

    g = Github(args.api_token)

    repo = g.get_repo("astral-sh/python-build-standalone")

    tag = args.tag if args.tag else get_latest_tag(repo)

    with progress(f"Getting release for tag {tag}..."):
        release = repo.get_release(tag)

    records = {}

    with progress("Getting release asset list..."):
        for asset in release.get_assets():
            name = asset.name
            url = asset.browser_download_url

            if not name.startswith("cpython-") or not name.endswith("-full.tar.zst"):
                continue

            # cpython-3.8.6+20220227-i686-pc-windows-msvc-shared-pgo-full.tar.zst

            parts = name.split("-")

            parts = parts[:-1]

            _python_flavor = parts.pop(0)
            version = parts.pop(0)
            python_version, tag = version.split("+", 1)
            major_minor = python_version.rsplit(".", 1)[0]

            if parts[-2] in ("shared", "static"):
                target_triple = "-".join(parts[0:-2])
                flavor = "-".join(parts[-2:])
            else:
                target_triple = "-".join(parts[0:-1])
                flavor = parts[-1]

            supports_prebuilt_extension_modules = (
                target_triple != "x86_64-unknown-linux-musl" and flavor != "static-noopt"
            )

            key = "%s-%s-%s" % (major_minor, target_triple, flavor)

            records[key] = {
                "name": name,
                "url": url,
                "major_minor": major_minor,
                "target_triple": target_triple,
                "supports_prebuilt_extension_modules": "true"
                if supports_prebuilt_extension_modules
                else "false",
            }

    print("// This Source Code Form is subject to the terms of the Mozilla Public")
    print("// License, v. 2.0. If a copy of the MPL was not distributed with this")
    print("// file, You can obtain one at https://mozilla.org/MPL/2.0/.")
    print()
    print("// THIS FILE IS AUTOGENERATED BY `scripts/fetch-python-distributions.py`.")
    print("// DO NOT EDIT MANUALLY.")
    print()
    print("//! Default Python distributions.")
    print()
    print("use crate::py_packaging::distribution::{")
    print("    PythonDistributionLocation, PythonDistributionRecord,")
    print("};")
    print("use crate::python_distributions::PythonDistributionCollection;")
    print("use once_cell::sync::Lazy;")
    print()
    print(
        "pub static PYTHON_DISTRIBUTIONS: Lazy<PythonDistributionCollection> = Lazy::new(|| {"
    )
    print("    let dists = vec![")

    lines = [
        "// Linux glibc linked.",
        format_record(records["3.9-aarch64-unknown-linux-gnu-noopt"]),
        format_record(records["3.9-x86_64-unknown-linux-gnu-pgo"]),
        format_record(records["3.9-x86_64_v2-unknown-linux-gnu-pgo"]),
        format_record(records["3.9-x86_64_v3-unknown-linux-gnu-pgo"]),
        format_record(records["3.10-aarch64-unknown-linux-gnu-noopt"]),
        format_record(records["3.10-x86_64-unknown-linux-gnu-pgo"]),
        format_record(records["3.10-x86_64_v2-unknown-linux-gnu-pgo"]),
        format_record(records["3.10-x86_64_v3-unknown-linux-gnu-pgo"]),
        format_record(records["3.11-aarch64-unknown-linux-gnu-noopt"]),
        format_record(records["3.11-x86_64-unknown-linux-gnu-pgo"]),
        format_record(records["3.11-x86_64_v2-unknown-linux-gnu-pgo"]),
        format_record(records["3.11-x86_64_v3-unknown-linux-gnu-pgo"]),
        format_record(records["3.12-aarch64-unknown-linux-gnu-noopt"]),
        format_record(records["3.12-x86_64-unknown-linux-gnu-pgo"]),
        format_record(records["3.12-x86_64_v2-unknown-linux-gnu-pgo"]),
        format_record(records["3.12-x86_64_v3-unknown-linux-gnu-pgo"]),
        format_record(records["3.13-aarch64-unknown-linux-gnu-noopt"]),
        format_record(records["3.13-x86_64-unknown-linux-gnu-pgo"]),
        format_record(records["3.13-x86_64_v2-unknown-linux-gnu-pgo"]),
        format_record(records["3.13-x86_64_v3-unknown-linux-gnu-pgo"]),
        "",
        "// Linux musl.",
        format_record(records["3.9-x86_64-unknown-linux-musl-noopt"]),
        format_record(records["3.9-x86_64_v2-unknown-linux-musl-noopt"]),
        format_record(records["3.9-x86_64_v3-unknown-linux-musl-noopt"]),
        format_record(records["3.10-x86_64-unknown-linux-musl-noopt"]),
        format_record(records["3.10-x86_64_v2-unknown-linux-musl-noopt"]),
        format_record(records["3.10-x86_64_v3-unknown-linux-musl-noopt"]),
        format_record(records["3.11-x86_64-unknown-linux-musl-noopt"]),
        format_record(records["3.11-x86_64_v2-unknown-linux-musl-noopt"]),
        format_record(records["3.11-x86_64_v3-unknown-linux-musl-noopt"]),
        format_record(records["3.12-x86_64-unknown-linux-musl-noopt"]),
        format_record(records["3.12-x86_64_v2-unknown-linux-musl-noopt"]),
        format_record(records["3.12-x86_64_v3-unknown-linux-musl-noopt"]),
        format_record(records["3.13-x86_64-unknown-linux-musl-noopt"]),
        format_record(records["3.13-x86_64_v2-unknown-linux-musl-noopt"]),
        format_record(records["3.13-x86_64_v3-unknown-linux-musl-noopt"]),
        "",
        "// The order here is important because we will choose the",
        "// first one. We prefer shared distributions on Windows because",
        "// they are more versatile: statically linked Windows distributions",
        "// don't declspec(dllexport) Python symbols and can't load shared",
        "// shared library Python extensions, making them a pain to work",
        "// with.",
        "",
        "// Windows shared.",
        format_record(records["3.9-i686-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.10-i686-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.11-i686-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.12-i686-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.13-i686-pc-windows-msvc-shared-pgo"]),
        "",
        format_record(records["3.9-x86_64-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.10-x86_64-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.11-x86_64-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.12-x86_64-pc-windows-msvc-shared-pgo"]),
        format_record(records["3.13-x86_64-pc-windows-msvc-shared-pgo"]),
        "",
        "// macOS.",
        format_record(records["3.9-aarch64-apple-darwin-pgo"]),
        format_record(records["3.10-aarch64-apple-darwin-pgo"]),
        format_record(records["3.11-aarch64-apple-darwin-pgo"]),
        format_record(records["3.12-aarch64-apple-darwin-pgo"]),
        format_record(records["3.13-aarch64-apple-darwin-pgo"]),
        format_record(records["3.9-x86_64-apple-darwin-pgo"]),
        format_record(records["3.10-x86_64-apple-darwin-pgo"]),
        format_record(records["3.11-x86_64-apple-darwin-pgo"]),
        format_record(records["3.12-x86_64-apple-darwin-pgo"]),
        format_record(records["3.13-x86_64-apple-darwin-pgo"]),
    ]

    for line in "\n".join(lines).splitlines(False):
        if line.strip():
            print("        %s" % line)
        else:
            print()

    print("    ];")
    print()
    print("    PythonDistributionCollection { dists }")
    print("});")


if __name__ == "__main__":
    main()
