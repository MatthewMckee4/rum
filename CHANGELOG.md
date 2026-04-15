# Changelog

## 0.1.0

Initial release.

- `rum <paths...>` removes files.
- `rum -r <paths...>` removes directories recursively.
- `rum -f` ignores missing paths.
- `rum -v` prints each removed path.
- Unix fast path: `openat` + `unlinkat` via rustix, with per-directory
  rayon fan-out.
