.. py:currentmodule:: oxidized_importer

.. _oxidized_importer_windows_in_memory_extensions:

==========================================================
Loading Extension Modules From Memory on Windows (3.13+)
==========================================================

This document describes why :py:class:`OxidizedFinder` cannot load
extension modules from memory on Windows with Python 3.13 or newer, what
guards are in place today, and a plan for restoring the feature. It is
meant to be the starting point for whoever resumes this work.

Status
======

As of November 2026:

* Loading extension modules (``.pyd`` files) from memory works on Windows
  with Python 3.12 and older.
* On Windows with Python 3.13 and newer, the feature is disabled:

  * PyOxidizer never places extension modules in memory for these
    versions, even if ``PythonPackagingPolicy.allow_in_memory_shared_library_loading``
    is ``True``. It logs a warning and uses the next allowed location
    (typically ``filesystem-relative:lib``) instead. This is implemented in
    ``importer_supports_in_memory_shared_library_loading()`` in
    ``pyoxidizer/src/py_packaging/standalone_builder.rs``.
  * If an extension module is nevertheless indexed for in-memory loading
    (e.g. via :py:class:`OxidizedResourceCollector` directly), importing it
    raises ``ImportError``. This is the ``#[cfg(all(windows, Py_3_13))]``
    variant of ``extension_module_shared_library_create_module()`` in
    ``python-oxidized-importer/src/importer.rs``.

* Loading extension modules from memory has never been supported on
  Linux or macOS.

How In-Memory Loading Works
===========================

CPython's own extension loader (``_imp.create_dynamic()``) only loads
shared libraries from files. To load from memory, ``oxidized_importer``
does the following in ``OxidizedFinder.create_module()``
(``python-oxidized-importer/src/importer.rs``):

1. Return the module from ``sys.modules`` if it is already there.
2. Map the DLL from memory with the MemoryModule library
   (``load_library_memory()`` in ``python-oxidized-importer/src/memory_dll.rs``).
   Imports of the DLL that are themselves in-memory resources are resolved
   from memory too.
3. Look up ``PyInit_<name>`` with ``get_proc_address_memory()`` and call it.
   This is ``load_dynamic_library()``, a reimplementation of CPython's
   ``_PyImport_LoadDynamicModuleWithSpec()``.
4. Turn the result into a module object:

   * Multi-phase initialization (PEP 489, ``PyInit_*`` returns a
     ``PyModuleDef``): ``PyModule_FromDefAndSpec()``. ``exec_module()``
     later calls ``_imp.exec_dynamic()``, which runs the ``Py_mod_exec``
     slots.
   * Single-phase initialization (``PyInit_*`` returns a module):
     ``PyModule_GetDef()``, store the init function in ``m_base.m_init``,
     then call ``_PyImport_FixupExtensionObject()``.

``_PyImport_FixupExtensionObject()`` does the bookkeeping CPython performs
for single-phase modules: it saves a copy of the module ``__dict__``
(``m_base.m_copy``), records the module in the interpreter's extension
cache, calls ``PyState_AddModule()`` and inserts the module into
``sys.modules``.

The 3.12 path already carries one compromise: CPython sets the package
context (``_Py_PackageContext``) around the ``PyInit_*`` call so that
``PyModule_Create()`` in a submodule like ``pkg._ext`` names the module
``pkg._ext`` instead of ``_ext``. PyO3 0.22.3 stopped exposing that
variable, so the code that set it is commented out in
``load_dynamic_library()``.

Why It Is Broken on Python 3.13+
================================

Python 3.13 reorganized extension module loading (``Python/importdl.c``
and ``Python/import.c``). The private functions the implementation needs
either disappeared or are no longer exported from ``pythonXY.dll``:

.. list-table::
   :header-rows: 1

   * - Symbol
     - 3.12
     - 3.13 / 3.14
   * - ``_PyImport_FixupExtensionObject``
     - ``PyAPI_FUNC`` in ``cpython/import.h`` (exported)
     - Removed. Replaced by static functions in ``import.c``
       (``update_global_state_for_extension()``,
       ``finish_singlephase_extension()``).
   * - ``_PyImport_GetModInitFunc``
     - Does not exist
     - Plain ``extern`` in ``internal/pycore_importdl.h``
   * - ``_PyImport_RunModInitFunc``
     - Does not exist
     - Plain ``extern`` in ``internal/pycore_importdl.h``
   * - ``_PyImport_SwapPackageContext``
     - Plain ``extern`` in ``internal/pycore_import.h``
     - Plain ``extern`` in ``internal/pycore_import.h``

On Windows, only functions declared with ``PyAPI_FUNC`` (i.e.
``__declspec(dllexport)``) are exported from ``pythonXY.dll``. Plain
``extern`` declarations in the internal headers are visible inside the
CPython build only. An extension cannot link against them, so the old
plan recorded in the comment above ``load_dynamic_library()`` (reimplement
``_PyImport_GetModInitFunc()`` and call ``_PyImport_RunModInitFunc()``
as-is) cannot work on Windows.

Plan for Restoring Support
==========================

The way forward is to stop depending on private CPython functions and
emulate the loading sequence with public API only. This is a new
``#[cfg(all(windows, Py_3_13))]`` implementation of
``load_dynamic_library()``; steps 1 to 3 above stay the same.

Multi-phase initialization
--------------------------

No private API is involved. Keep the existing
``PyModule_FromDefAndSpec()`` branch. ``exec_module()`` already calls
``_imp.exec_dynamic()``. Most modern extensions use multi-phase
initialization (e.g. Cython 3 generates it by default), so this case
alone covers a lot.

Single-phase initialization
---------------------------

After ``PyInit_*`` returns a module:

1. ``PyModule_GetDef()``; fail with ``SystemError`` if it returns ``NULL``,
   as today.
2. Set ``m_base.m_init`` to the init function, as today.
3. Call ``PyState_AddModule(module, def)``. This is public API and is what
   makes ``PyState_FindModule()`` work in the extension.
4. Insert the module into ``sys.modules`` under its fully qualified name.
5. Work around the missing package context: if ``PyModule_GetName()`` is
   not the fully qualified name, set the module's ``__name__`` to the fully
   qualified name. Types created during initialization keep the short
   ``__module__``; this already happens on 3.12.

Known differences from CPython for single-phase modules:

* No ``m_base.m_copy`` and no entry in the extension cache. If code deletes
  the module from ``sys.modules`` and imports it again, ``PyInit_*`` runs a
  second time instead of the module being restored from the saved copy.
  Many single-phase extensions keep C-level static state and may misbehave
  or leak when initialized twice. Normal applications do not do this.
* Loading the same single-phase module in a sub-interpreter is not
  supported (it is not supported by CPython either, except via the
  extension cache).

Before writing the implementation, re-read ``import_run_extension()`` in
CPython's ``Python/import.c`` for the target versions and check whether
3.14 added public replacements (e.g. ``PyModule_FromSlotsAndSpec()`` or
other PEP 793 related API) that could replace part of the above.

Testing
-------

There is no automated test for in-memory extension loading today. Before
removing the guards, add a pyembed test that runs on the Windows CI
runners (``.github/workflows/workspace-python.yml`` already tests
``x86_64-pc-windows-msvc`` and ``i686-pc-windows-msvc``):

* Take a ``.pyd`` from the test interpreter's ``DLLs`` directory, add it to
  an :py:class:`OxidizedResourceCollector` with ``add_in_memory()``,
  install the resulting :py:class:`OxidizedFinder`, import the module and
  call a function from it.
* Cover both a multi-phase and a single-phase extension. Check which one a
  given ``.pyd`` uses rather than assuming; a small single-phase test
  extension may have to be built as part of the test (for example with
  PyO3, which generates single-phase initialization).
* Cover an extension inside a package (``pkg._ext``) to exercise the
  ``__name__`` workaround.

Removing the guards
-------------------

Once the implementation passes on Windows CI:

1. Make ``importer_supports_in_memory_shared_library_loading()`` in
   ``pyoxidizer/src/py_packaging/standalone_builder.rs`` return ``true`` for
   the supported versions, and update
   ``test_windows_dynamic_extension_in_memory_unsupported_python_version``.
2. Remove the ``ImportError`` stub in ``importer.rs``.
3. Update :ref:`oxidized_importer_common_issues`, the PyOxidizer and
   ``oxidized_importer`` changelogs, and this document.
