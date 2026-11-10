.. py:currentmodule:: oxidized_importer

.. _oxidized_importer_history:

===============
Project History
===============

Changelog
=========

.. next-release

Next
----

(Not yet released)

* :py:class:`OxidizedResourceReader` now implements ``files()``, returning a
  new :py:class:`OxidizedResourceTraversable` that implements
  ``importlib.resources.abc.Traversable`` for package resources.
  Previously, ``importlib.resources`` fell back to its own compatibility
  wrapper, whose ``read_text()`` does not accept ``errors``. Since Python
  3.13, ``importlib.resources.read_text()`` always passes ``errors`` to
  ``Traversable.read_text()``, so every call to it failed with
  ``TypeError: Traversable.read_text() got an unexpected keyword argument
  'errors'`` for resources loaded by ``oxidized_importer``. Traversing into
  sub-directories and child packages via ``joinpath()`` and ``/`` is
  supported, and ``iterdir()`` now lists resource sub-directories and child
  packages instead of every resource path in the package.
* Package resources stored in a filesystem-relative location are now
  returned as ``pathlib.Path`` instances by
  :py:meth:`OxidizedResourceTraversable.joinpath`, the ``/`` operator and
  :py:meth:`OxidizedResourceTraversable.iterdir`.
  ``importlib.resources.as_file()`` used to copy such resources to a
  temporary file. Code that loads a bundled shared library this way, such
  as ``libusb_package``, received a new temporary copy on every start. It
  now gets the installed file.
  :py:meth:`OxidizedResourceReader.resource_path` returns the path of these
  resources instead of always raising ``FileNotFoundError``.
* Importing an extension module indexed for in-memory loading on Windows
  with Python 3.13 or newer now raises ``ImportError`` instead of
  panicking. See :ref:`oxidized_importer_windows_in_memory_extensions`
  for why this is not supported and how support could be restored.

0.10.0
------

Released 2025-10-05

* PyO3 upgraded from 0.17 to 0.25.1.
* Added support for Python versions up to and including 3.12.
* Disable in-memory extension import for Python 3.13 on Windows until
  we can make it work again. See
  :ref:`oxidized_importer_windows_in_memory_extensions` for details.

0.9.0
-----

Released 2022-12-29.

0.8.0
-----

Released November 7, 2022.

0.7.0
-----

No release notes.

0.6.0
-----

Released on June 6, 2022.

* Added missing API docs for :py:class:`OxidizedDistribution`.
* ``OxidizedDistribution.metadata`` now returns an
  ``importlib.metadata._adapters.Message`` instance on Python 3.10+.
* ``OxidizedDistribution.entry_points`` now calls
  ``importlib.metadata.EntryPoints._from_text_for`` on Python 3.10+.
  Previously, the implementation of this method didn't work properly on 3.10+.
* Added ``name`` property to :py:class:`OxidizedDistribution`.
* Added ``_normalized_name`` property to :py:class:`OxidizedDistribution`.
* PyO3 Rust crate upgraded to 0.16.5. This gets us better compatibility with
  Python 3.10.
