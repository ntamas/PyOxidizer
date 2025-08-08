# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import _imp

import os
import pathlib
import sys
import tempfile
import unittest

from oxidized_importer import (
    OxidizedResourceCollector,
    OxidizedFinder,
    find_resources_in_path,
)


class TestImporterConstruction(unittest.TestCase):
    def setUp(self):
        self.raw_temp_dir = tempfile.TemporaryDirectory(
            prefix="oxidized_importer-test-"
        )
        self.td = pathlib.Path(self.raw_temp_dir.name)

    def tearDown(self):
        self.raw_temp_dir.cleanup()
        del self.raw_temp_dir
        del self.td

    def get_resources_data(self) -> bytes:
        c = OxidizedResourceCollector(allowed_locations=["in-memory"])

        for path in sys.path:
            if os.path.isdir(path):
                for i, resource in enumerate(find_resources_in_path(path)):
                    c.add_in_memory(resource)

                    if i == 10:
                        break

        finder = OxidizedFinder()
        finder.add_resources(c.oxidize()[0])

        return finder.serialize_indexed_resources()

    def test_index_interpreter_builtins(self):
        f = OxidizedFinder()
        f.index_interpreter_builtins()

    def test_index_interpreter_builtin_extension_modules(self):
        f = OxidizedFinder()
        f.index_interpreter_builtin_extension_modules()

    def test_index_interpreter_frozen_modules(self):
        f = OxidizedFinder()
        f.index_interpreter_frozen_modules()

        if sys.version_info >= (3, 11):
            # Compare the list of indexed modules with _imp.frozen_modules
            indexed_frozen = set(r.name for r in f.indexed_resources() if r.is_frozen_module)
            real_frozen =  set(_imp._frozen_module_names())  # type: ignore

            diff = sorted(indexed_frozen - real_frozen)
            if diff:
                self.fail(f"Modules indexed as frozen but are not frozen: {diff!r}")

            diff = sorted(real_frozen - indexed_frozen)
            if diff:
                self.fail(f"Frozen modules not indexed: {diff!r}")

    def test_index_bytes_bad(self):
        f = OxidizedFinder()

        with self.assertRaises(ValueError):
            f.index_bytes(b"foo")

    def test_index_bytes_simple(self):
        f = OxidizedFinder()

        f.index_bytes(self.get_resources_data())

    def test_index_file_memory_mapped_no_file(self):
        f = OxidizedFinder()

        with self.assertRaises(ValueError):
            f.index_file_memory_mapped(self.td / "does-not-exist")

    def test_index_file_memory_mapped_simple(self):
        path = self.td / "simple"

        with path.open("wb") as fh:
            fh.write(self.get_resources_data())

        f = OxidizedFinder()
        f.index_file_memory_mapped(path)


if __name__ == "__main__":
    unittest.main()
