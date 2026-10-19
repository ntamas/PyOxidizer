#!/usr/bin/env python3
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""Emit default_python_distributions.rs boilerplate for python-build-standalone release.

The SHA-256 digests are taken from the ``SHA256SUMS`` file published with
each release, so no distribution archives need to be downloaded.
"""

import argparse
import json
import os
import re
import socket
import sys
import urllib.parse
import urllib.request

from contextlib import contextmanager

try:
    import certifi
except ImportError:
    certifi = None


REPO = "astral-sh/python-build-standalone"

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

RE_CAL_VER = re.compile(r"^\d{8}$")

# Python versions to emit distributions for.
PYTHON_VERSIONS = ["3.10", "3.11", "3.12", "3.13", "3.14"]

# Linux (glibc) and macOS optimized builds are only published as `pgo+lto`.
# Their libpython consists of LLVM bitcode, which PyOxidizer links with LLD.
LINUX_GNU_TARGETS = [
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    "x86_64_v2-unknown-linux-gnu",
    "x86_64_v3-unknown-linux-gnu",
]

# The `+static` musl builds are fully statically linked and can only use
# built-in extension modules. The non-static musl builds dynamically link
# against musl and are not what PyOxidizer users expect from musl targets.
LINUX_MUSL_TARGETS = [
    "x86_64-unknown-linux-musl",
    "x86_64_v2-unknown-linux-musl",
    "x86_64_v3-unknown-linux-musl",
]

# Windows builds dynamically link libpython (there is no static flavor any
# more) and can load shared library extension modules.
WINDOWS_TARGETS = [
    "i686-pc-windows-msvc",
    "x86_64-pc-windows-msvc",
]

MACOS_TARGETS = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
]


def urlopen(request):
    # Python installs without a CA bundle (e.g. the python.org macOS
    # installer) can't verify certificates unless certifi is used.
    if certifi:
        import ssl

        context = ssl.create_default_context(cafile=certifi.where())
        return urllib.request.urlopen(request, context=context)

    return urllib.request.urlopen(request)


@contextmanager
def progress(message):
    print(message, end=" ", file=sys.stderr, flush=True)
    yield
    print("done.", file=sys.stderr)


def github_request(url, token=None):
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "pyoxidizer-fetch-python-distributions",
    }
    if token:
        headers["Authorization"] = f"Bearer {token}"

    with urlopen(urllib.request.Request(url, headers=headers)) as r:
        return json.load(r)


def get_latest_tag(token=None):
    """Get the tag of the latest release."""
    with progress("Finding latest release..."):
        release = github_request(
            f"https://api.github.com/repos/{REPO}/releases/latest", token
        )

    tag = release["tag_name"]
    if not RE_CAL_VER.match(tag):
        raise Exception(f"unexpected release tag: {tag}")

    return tag


def asset_url(tag, name):
    return "https://github.com/%s/releases/download/%s/%s" % (
        REPO,
        tag,
        urllib.parse.quote(name, safe=""),
    )


def get_sha256sums(tag):
    """Obtain a mapping of asset name to SHA-256 digest for a release."""
    with progress(f"Downloading SHA256SUMS for release {tag}..."):
        with urlopen(asset_url(tag, "SHA256SUMS")) as r:
            data = r.read().decode("utf-8")

    sums = {}

    for line in data.splitlines():
        line = line.strip()
        if not line:
            continue

        digest, name = line.split(None, 1)
        sums[name.lstrip("*")] = digest

    return sums


def parse_asset_name(name):
    """Parse a distribution archive name into its components.

    Returns ``(major_minor, target_triple, flavor)`` or ``None`` if the asset
    isn't a full CPython distribution archive.
    """
    # We are only interested in CPython, and we always want full builds,
    # not the install-only variants.
    if not name.startswith("cpython-") or not name.endswith("-full.tar.zst"):
        return None

    # e.g. cpython-3.13.16+20261003-x86_64-unknown-linux-gnu-pgo+lto-full.tar.zst
    # or   cpython-3.8.6+20220227-i686-pc-windows-msvc-shared-pgo-full.tar.zst
    parts = name[: -len("-full.tar.zst")].split("-")

    parts.pop(0)
    version = parts.pop(0)
    python_version, _tag = version.split("+", 1)
    major_minor = python_version.rsplit(".", 1)[0]

    # Older releases named Windows flavors `shared-pgo` / `static-noopt`.
    if len(parts) > 1 and parts[-2] in ("shared", "static"):
        target_triple = "-".join(parts[0:-2])
        flavor = "-".join(parts[-2:])
    else:
        target_triple = "-".join(parts[0:-1])
        flavor = parts[-1]

    return major_minor, target_triple, flavor


def bool_to_str(value):
    return "true" if value else "false"


def main():
    socket.setdefaulttimeout(60)

    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--api-token",
        help="GitHub API token (defaults to GITHUB_TOKEN); only needed to look up the latest release",
        default=os.environ.get("GITHUB_TOKEN"),
    )
    parser.add_argument(
        "--tag", help="python-build-standalone release tag, default is latest", default=None
    )

    args = parser.parse_args()

    tag = args.tag if args.tag else get_latest_tag(args.api_token)

    records = {}

    for name, sha256 in get_sha256sums(tag).items():
        parsed = parse_asset_name(name)
        if parsed is None:
            continue

        major_minor, target_triple, flavor = parsed

        # Statically linked distributions can't load shared library extension
        # modules.
        static = flavor.endswith("+static") or flavor.startswith("static-")

        key = "%s-%s-%s" % (major_minor, target_triple, flavor)

        records[key] = {
            "name": name,
            "url": asset_url(tag, name),
            "sha256": sha256,
            "major_minor": major_minor,
            "target_triple": target_triple,
            "supports_prebuilt_extension_modules": bool_to_str(not static),
        }

    def record(major_minor, target_triple, flavor):
        key = "%s-%s-%s" % (major_minor, target_triple, flavor)
        if key not in records:
            raise Exception(f"release {tag} does not provide {key}")

        return ENTRY.format(**records[key])

    def group(targets, flavor):
        return [
            record(major_minor, target_triple, flavor)
            for major_minor in PYTHON_VERSIONS
            for target_triple in targets
        ]

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

    lines = (
        ["// Linux glibc linked."]
        + group(LINUX_GNU_TARGETS, "pgo+lto")
        + ["", "// Linux musl (statically linked)."]
        + group(LINUX_MUSL_TARGETS, "noopt+static")
        + ["", "// Windows (dynamically linked libpython)."]
        + group(WINDOWS_TARGETS, "pgo")
        + ["", "// macOS."]
        + group(MACOS_TARGETS, "pgo+lto")
    )

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
