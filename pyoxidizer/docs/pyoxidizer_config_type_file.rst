.. py:currentmodule:: starlark_pyoxidizer

========
``File``
========

.. py:class:: File

    This type represents a concrete file in an abstract filesystem. The
    file has a path and content.

    Instances can be constructed by calling methods that emit resources
    with a :py:class:`PythonPackagingPolicy` having
    :py:attr:`PythonPackagingPolicy.file_scanner_emit_files` set to ``True``.
    Files in ``<name>.libs`` directories of wheels are always emitted as
    instances of this type.

    .. py:attribute:: name

        (``string``)

        Alias of :py:attr:`path`. Every resource type has a ``name``
        attribute, and resource callbacks can read it without checking the
        type of the resource.

    .. py:attribute:: path

        (``string``)

        The filesystem path represented. Typically relative. Doesn't
        have to correspond to a valid, existing file on the filesystem.

    .. py:attribute:: is_executable

        (``bool``)

        Whether the file is executable.

    .. py:attribute:: is_*

        (various)

        See :ref:`config_resource_add_attributes`.
