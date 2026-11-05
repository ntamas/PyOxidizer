# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import importlib.resources
import io
import os
import pathlib
import sys
import tempfile
import unittest

from oxidized_importer import (
    OxidizedFinder,
    OxidizedResourceCollector,
    OxidizedResourceReader,
    OxidizedResourceTraversable,
    find_resources_in_path,
)


class TestImporterResourceReading(unittest.TestCase):
    def setUp(self):
        self.raw_temp_dir = tempfile.TemporaryDirectory(
            prefix="oxidized_importer-test-"
        )
        self.td = pathlib.Path(self.raw_temp_dir.name)

    def tearDown(self):
        self.raw_temp_dir.cleanup()
        del self.raw_temp_dir
        del self.td

    def _make_package(self, name):
        package_path = self.td

        for part in name.split("."):
            package_path = package_path / part

            package_path.mkdir(exist_ok=True)

            with (package_path / "__init__.py").open("wb"):
                pass

        return package_path

    def _finder_from_td(self):
        collector = OxidizedResourceCollector(allowed_locations=["in-memory"])
        for r in find_resources_in_path(self.td):
            collector.add_in_memory(r)

        f = OxidizedFinder()
        f.add_resources(collector.oxidize()[0])

        return f

    def _install_finder(self, f, package):
        sys.meta_path.insert(0, f)

        def cleanup():
            sys.meta_path.remove(f)
            for name in list(sys.modules):
                if name == package or name.startswith(package + "."):
                    del sys.modules[name]

        self.addCleanup(cleanup)

    def _make_traversable_package(self):
        p = self._make_package("oxidized_traversable.sub")
        p = p.parent

        with (p / "resource.txt").open("wb") as fh:
            fh.write(b"caf\xc3\xa9")

        (p / "child0").mkdir()
        with (p / "child0" / "a.txt").open("wb") as fh:
            fh.write(b"a")

        with (p / "sub" / "sub.txt").open("wb") as fh:
            fh.write(b"sub")

        return self._finder_from_td()

    def test_get_resource_reader_missing_package(self):
        f = self._finder_from_td()
        self.assertIsNone(f.get_resource_reader("my_package"))

    def test_get_resource_reader_not_package(self):
        with (self.td / "my_package.py").open("wb"):
            pass

        f = self._finder_from_td()

        self.assertIsNone(f.get_resource_reader("my_package"))

    def test_top_level_package(self):
        p = self._make_package("my_package")

        with (p / "resource.txt").open("wb") as fh:
            fh.write(b"my resource")

        f = self._finder_from_td()

        entries = [r for r in f.indexed_resources() if r.name == "my_package"]
        self.assertEqual(len(entries), 1)
        self.assertTrue(entries[0].is_package)

        r = f.get_resource_reader("my_package")

        self.assertIsInstance(r, OxidizedResourceReader)

        with self.assertRaises(FileNotFoundError):
            r.is_resource("missing")

        self.assertTrue(r.is_resource("resource.txt"))

        contents = r.contents()
        self.assertIsInstance(contents, list)
        self.assertEqual(contents, ["resource.txt"])

        with self.assertRaises(FileNotFoundError):
            r.resource_path("resource.txt")

        with self.assertRaises(FileNotFoundError):
            r.open_resource("missing")

        f = r.open_resource("resource.txt")
        self.assertIsInstance(f, io.BytesIO)
        self.assertEqual(f.getvalue(), b"my resource")

    def test_child_directory(self):
        p = self._make_package("my_package")

        child0_path = p / "child0"
        child1_path = p / "child1"

        child0_path.mkdir()
        child1_path.mkdir()

        with (child0_path / "a.txt").open("wb") as fh:
            fh.write(b"a")
        with (child1_path / "b.txt").open("wb") as fh:
            fh.write(b"b")

        f = self._finder_from_td()
        r = f.get_resource_reader("my_package")

        self.assertIsInstance(r, OxidizedResourceReader)

        self.assertTrue(r.is_resource("child0/a.txt"))
        self.assertTrue(r.is_resource("child1/b.txt"))

        self.assertEqual(r.contents(), ["child0/a.txt", "child1/b.txt"])

        self.assertEqual(r.open_resource("child0/a.txt").getvalue(), b"a")
        self.assertEqual(r.open_resource("child1/b.txt").getvalue(), b"b")

    def test_files(self):
        f = self._make_traversable_package()
        root = f.get_resource_reader("oxidized_traversable").files()

        self.assertIsInstance(root, OxidizedResourceTraversable)
        self.assertEqual(root.name, "oxidized_traversable")
        self.assertTrue(root.is_dir())
        self.assertFalse(root.is_file())
        self.assertEqual(
            sorted(p.name for p in root.iterdir()), ["child0", "resource.txt", "sub"]
        )

        resource = root / "resource.txt"
        self.assertEqual(resource.name, "resource.txt")
        self.assertTrue(resource.is_file())
        self.assertFalse(resource.is_dir())
        self.assertEqual(resource.read_bytes(), b"caf\xc3\xa9")
        self.assertEqual(resource.read_text(encoding="utf-8"), "caf\xe9")
        self.assertEqual(
            resource.read_text(encoding="ascii", errors="replace"),
            "caf\ufffd\ufffd",
        )
        with self.assertRaises(UnicodeDecodeError):
            resource.read_text(encoding="ascii")
        with resource.open("rb") as fh:
            self.assertEqual(fh.read(), b"caf\xc3\xa9")
        with resource.open("r", encoding="utf-8") as fh:
            self.assertEqual(fh.read(), "caf\xe9")
        with resource.open(encoding="ascii", errors="ignore") as fh:
            self.assertEqual(fh.read(), "caf")
        with self.assertRaises(ValueError):
            resource.open("w")
        with self.assertRaises(NotADirectoryError):
            resource.iterdir()

        child0 = root.joinpath("child0")
        self.assertTrue(child0.is_dir())
        self.assertFalse(child0.is_file())
        self.assertEqual([p.name for p in child0.iterdir()], ["a.txt"])
        self.assertEqual(root.joinpath("child0", "a.txt").read_bytes(), b"a")
        self.assertEqual(root.joinpath("child0/a.txt").read_bytes(), b"a")

        sub = root / "sub"
        self.assertTrue(sub.is_dir())
        self.assertEqual([p.name for p in sub.iterdir()], ["sub.txt"])
        self.assertEqual((sub / "sub.txt").read_text(encoding="utf-8"), "sub")
        self.assertEqual(root.joinpath("sub/sub.txt").read_bytes(), b"sub")

        missing = root / "missing"
        self.assertFalse(missing.is_file())
        self.assertFalse(missing.is_dir())
        with self.assertRaises(FileNotFoundError):
            missing.open("rb")
        with self.assertRaises(FileNotFoundError):
            missing.read_bytes()
        with self.assertRaises(FileNotFoundError):
            missing.iterdir()
        with self.assertRaises(IsADirectoryError):
            root.open("rb")

    def test_importlib_resources(self):
        f = self._make_traversable_package()
        self._install_finder(f, "oxidized_traversable")

        files = importlib.resources.files("oxidized_traversable")
        self.assertIsInstance(files, OxidizedResourceTraversable)
        self.assertEqual(
            files.joinpath("child0/a.txt").read_text(encoding="utf-8"), "a"
        )

        # read_text() in Python 3.13+ forwards errors to Traversable.read_text().
        self.assertEqual(
            importlib.resources.read_text(
                "oxidized_traversable",
                "resource.txt",
                encoding="ascii",
                errors="replace",
            ),
            "caf\ufffd\ufffd",
        )
        self.assertEqual(
            importlib.resources.read_text(
                "oxidized_traversable", "resource.txt", encoding="utf-8"
            ),
            "caf\xe9",
        )

        with importlib.resources.as_file(files / "resource.txt") as path:
            self.assertEqual(path.read_bytes(), b"caf\xc3\xa9")


if __name__ == "__main__":
    unittest.main()
