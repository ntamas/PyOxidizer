# Notes

## Crates that cannot be updated

### `pyo3-ffi`

`pyo3-ffi` is stuck at 0.24.2 because version 0.25.0 removed the following
symbols, which are needed in `python-oxidized-importer`:

* `_PyImport_FrozenBootstrap`
* `_PyImport_FrozenStdlib`
* `_PyImport_FrozenTest`

### `rcgen`

`rcgen` is stuck at 0.12.1 at the moment because the API was refactored
significantly and we would need to re-write `tugger-code-signing` to
cater for the new conventions.

### `starlark`

Starlark API underwent a significant refactoring after 0.3.2 and we did
not update our code to reflect the changes.

