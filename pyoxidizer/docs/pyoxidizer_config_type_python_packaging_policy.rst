.. py:currentmodule:: starlark_pyoxidizer

=========================
``PythonPackagingPolicy``
=========================

.. py:class:: PythonPackagingPolicy

    When building a Python binary, there are various settings that control which
    Python resources are added, where they are imported from, and other various
    settings. This collection of settings is referred to as a *Python Packaging
    Policy*. These settings are represented by the ``PythonPackagingPolicy`` type.

    .. py:attribute:: allow_files

        (``bool``)

        Whether to allow the collection of generic *file* resources.

        If false, all collected/packaged resources must be instances of
        concrete resource types (``PythonModuleSource``, ``PythonPackageResource``,
        etc).

        If true, :py:class:`File` instances can be added to resource
        collectors.

    .. py:attribute:: allow_in_memory_shared_library_loading

        (``bool``)

        Whether to allow loading of Python extension modules and shared libraries
        from memory at run-time.

        Some platforms (notably Windows) allow opening shared libraries from a
        memory address. This mode of opening shared libraries allows libraries
        to be embedded in binaries without having to statically link them. However,
        not every library works correctly when loaded this way.

        This flag defines whether to enable this feature where supported. Its
        true value can be ignored if the target platform doesn't support loading
        shared library from memory.

        Loading extension modules from memory is currently only supported on
        Windows with Python 3.12 or older. With Python 3.13 or newer, this flag
        is ignored and a warning is logged.

    .. py:attribute:: bytecode_optimize_level_zero

        (``bool``)

        Whether to add Python bytecode at optimization level 0 (the
        default optimization level the Python interpreter compiles bytecode for).

    .. py:attribute:: bytecode_optimize_level_one

        (``bool``)

        Whether to add Python bytecode at optimization level 1.

    .. py:attribute:: bytecode_optimize_level_two

        (``bool``)

        Whether to add Python bytecode at optimization level 2.

    .. py:attribute:: dunder_file_allowed_packages

        (``list<string>``)

        Packages whose modules are known to work without ``__file__``.

        When building, PyOxidizer looks for embedded modules whose code reads
        ``__file__``, which is not set for modules imported from memory.
        Modules in the standard library and modules in the packages listed
        here only produce a note. Any other module reading ``__file__``
        produces a warning that lists the affected modules. In GitHub Actions,
        the warning is also emitted as an annotation on the job.

        Each entry covers the named package or module and all modules below
        it. For example, ``["foo"]`` covers ``foo`` and ``foo.bar`` but not
        ``foobar``.

        Assigning ``None`` clears the list.

        Default is an empty list.

    .. py:attribute:: excluded_packages

        (``list<string>``)

        Packages whose resources are left out of the build.

        Each entry covers the named package or module and everything below
        it, including the distribution metadata of a distribution with the
        same name. For example, ``["foo"]`` covers ``foo`` and ``foo.bar`` but
        not ``foobar``.

        Extension modules are not affected, because PyOxidizer currently adds
        them regardless of their ``add_include`` attribute.

        Assigning ``None`` clears the list.

        Default is an empty list.

    .. py:attribute:: extension_module_filter

        (``string``)

        The filter to apply to determine which extension modules to add.
        The following values are recognized:

        ``all``
          Every named extension module will be included.

        ``minimal``
          Return only extension modules that are required to initialize a
          Python interpreter. This is a very small set and various functionality
          from the Python standard library will not work with this value.

        ``no-libraries``
          Return only extension modules that don't require any additional libraries.

          Most common Python extension modules are included. Extension modules
          like ``_ssl`` (links against OpenSSL) and ``zlib`` are not included.

        ``no-copyleft``
          Return only extension modules that do not link against *copyleft* licensed
          libraries.

          Not all Python distributions may annotate license info for all extensions
          or the libraries they link against. If license info is missing, the
          extension is not included because it *could* be *copyleft* licensed.
          Similarly, the mechanism for determining whether a license is *copyleft* is
          based on the SPDX license annotations, which could be wrong or out of date.

        Default is ``all``.

    .. py:attribute:: file_scanner_classify_files

        (``bool``)

        Whether file scanning should attempt to classify files and emit typed
        resources corresponding to the detected file type.

        If ``True``, operations that emit resource objects (such as
        :py:meth:`PythonExecutable.pip_install`) will emit specific
        types for each resource flavor. e.g. :py:class:`PythonModuleSource`,
        :py:class:`PythonExtensionModule`, etc.

        If ``False``, the file scanner does not attempt to classify the type of
        a file and this rich resource types are not emitted.

        Can be used in conjunction with
        :py:attr:`PythonPackagingPolicy.file_scanner_emit_files`. If both
        are ``True``, there will be a :py:class:`File` and an optional non-file
        resource for each source file.

        Default is ``True``.

    .. py:attribute:: file_scanner_emit_files

        (``bool``)

        Whether file scanning should emit file resources for each seen file.

        If ``True``, operations that emit resource objects (such as
        :py:meth:`PythonExecutable.pip_install`) will emit
        :py:class:`File` instances for each encountered file.

        If ``False``, :py:class:`File` instances will not be emitted.

        Can be used in conjunction with
        :py:attr:`PythonPackagingPolicy.file_scanner_classify_files`.

        Default is ``False``.

    .. py:attribute:: filesystem_packages

        (``list<string>``)

        Packages whose resources are placed on the filesystem.

        Use this for packages that do not work when imported from memory,
        e.g. because they need ``__file__`` or their own source code. The
        resources of these packages are placed at
        :py:attr:`PythonPackagingPolicy.filesystem_packages_location`, without
        a fallback location. The source code of their modules is always
        stored, regardless of
        :py:attr:`PythonPackagingPolicy.include_non_distribution_sources`.

        Each entry covers the named package or module and everything below
        it, including the distribution metadata of a distribution with the
        same name. For example, ``["foo"]`` covers ``foo`` and ``foo.bar`` but
        not ``foobar``.

        Assigning ``None`` clears the list.

        Default is an empty list.

    .. py:attribute:: filesystem_packages_location

        (``string``)

        The location of the resources of
        :py:attr:`PythonPackagingPolicy.filesystem_packages`. It must be a
        ``filesystem-relative:<prefix>`` location.

        Default is ``filesystem-relative:lib``.

    .. py:attribute:: include_classified_resources

        (``bool``)

        Whether strongly typed, classified non-:py:class:`File` resources have their
        ``add_include`` attribute set to ``True`` by default.

        Default is ``True``.

    .. py:attribute:: include_distribution_sources

        (``bool``)

        Whether to add source code for Python modules in the Python
        distribution.

        Default is ``True``.

    .. py:attribute:: include_distribution_resources

        (``bool``)

        Whether to add Python package resources for Python packages
        in the Python distribution.

        Default is ``False``.

    .. py:attribute:: include_file_resources

        (``bool``)

        Whether :py:class:`File` resources have their ``add_include`` attribute
        set to ``True`` by default.

        Default is ``False``.

    .. py:attribute:: include_non_distribution_sources

        (``bool``)

        Whether to add source code for Python modules not in the Python
        distribution.

    .. py:attribute:: include_test

        (``bool``)

        Whether to add Python resources related to tests.

        Not all files associated with tests may be properly flagged as such.
        This is a best effort setting. By default only the test packages of
        the standard library are flagged. Use
        :py:attr:`PythonPackagingPolicy.test_package_names` to flag the tests
        of other packages.

        Default is ``False``.

    .. py:attribute:: resources_location

        (``string``)

        The location that resources should be added to by default.

        Default is ``in-memory``.

    .. py:attribute:: resources_location_fallback

        (``string`` or ``None``)

        The fallback location that resources should be added to if
        ``resources_location`` fails.

        Default is ``None``.

    .. py:attribute:: preferred_extension_module_variants

        (``dict<string, string>``) (readonly)

        Mapping of extension module name to variant name.

        This mapping defines which preferred named variant of an extension module
        to use. Some Python distributions offer multiple variants of the same
        extension module. This mapping allows defining which variant of which
        extension to use when choosing among them.

        Keys set on this dict are not reflected in the underlying policy. To set
        a key, call the ``set_preferred_extension_module_variant()`` method.

    .. py:attribute:: python_platform_compatibility_tag_override

        (``list<string>`` or ``None``)

        Platform compatibility tags passed to ``pip download`` (via its
        ``--platform`` argument) when resolving wheels for the target Python
        distribution, overriding the tag derived from the distribution (e.g.
        ``manylinux2014_x86_64`` on Linux or ``macosx_10_9_x86_64`` on macOS).

        Use this when packages you depend on only publish wheels for newer
        platform tags than the distribution's default, such as
        ``manylinux_2_28_x86_64`` or a newer macOS deployment target. Wheels
        matching any of the listed tags are accepted, so list the older tag as
        well if some of your dependencies still only publish wheels for it::

            policy.python_platform_compatibility_tag_override = [
                "manylinux_2_28_x86_64",
                "manylinux2014_x86_64",
            ]

        A single ``string`` can be assigned as a shorthand for a one-element
        list. Reading the attribute always yields a list, or ``None`` if no
        override is set.

        Default is ``None``.

    .. py:attribute:: test_package_names

        (``list<string>``)

        Package names that mark resources as tests.

        A resource is a test if any package in its name has one of these
        names. For example, ``["tests"]`` marks ``foo.tests``,
        ``foo.tests.test_bar`` and the package resources in ``foo.tests``.
        Only package names count: a module that is not a package is never a
        test by its own name. This keeps runtime modules such as
        ``jinja2.tests`` or ``werkzeug.test``. When the list is not empty,
        ``conftest`` modules are also tests.

        Resources marked here are handled like the standard library test
        packages: they are left out unless
        :py:attr:`PythonPackagingPolicy.include_test` is ``True``.

        A typical value is
        ``["test", "tests", "_test", "_tests", "benchmarks"]``.

        Assigning ``None`` clears the list.

        Default is an empty list.


    .. py:method:: register_resource_callback(f: Callable)

        This method registers a Starlark function to be called when resource objects
        are created. The passed function receives 2 arguments: this
        ``PythonPackagingPolicy`` instance and the resource (e.g.
        ``PythonModuleSource``) that was created.

        The purpose of the callback is to enable Starlark configuration files to
        mutate resources upon creation so they can globally influence how those
        resources are packaged.

    .. py:method:: set_preferred_extension_module_variant(extension: str, variant: str)

        This method will set a preferred Python extension module variant to
        use. See the documentation for ``preferred_extension_module_variants``
        above for more.

        It accepts 2 ``string`` arguments defining the extension module name
        and its preferred variant.

    .. py:method:: set_resource_handling_mode(mode: str)

        This method takes a string argument denoting the *resource handling mode*
        to apply to the policy. This string can have the following values:

        ``classify``
           Files are classified as typed resources and handled as such.

           Only classified resources can be added by default.

        ``files``
           Files are handled as raw files (as opposed to typed resources).

           Only files can be added by default.

        This method is effectively a convenience method for bulk-setting
        multiple attributes on the instance given a behavior mode.

        ``classify`` will configure the file scanner to emit classified resources,
        configure the ``add_include`` attribute to only be ``True`` on classified
        resources, and will disable the addition of :py:class:`File` resources on resource
        collectors.

        ``files`` will configure the file scanner to only emit :py:class:`File` resources,
        configure the ``add_include`` attribute to ``True`` on :py:class:`File` and *classified*
        resources, and will allow resource collectors to add :py:class:`File` instances.
