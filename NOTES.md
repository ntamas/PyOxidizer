# Notes

## Crates that cannot be updated

### `rcgen`

`rcgen` is stuck at 0.12.1 at the moment because the API was refactored
significantly and we would need to re-write `tugger-code-signing` to
cater for the new conventions.

### `starlark`

Starlark API underwent a significant refactoring after 0.3.2 and we did
not update our code to reflect the changes.

