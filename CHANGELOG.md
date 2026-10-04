# Changelog


## [0.1.0] - 2026-10-04

- Initial release of the crate. The crate contains struct definitions needed for parsing and
  serialization of DASH MPD manifests, which use a dedicated XML schema. The definitions have been
  split out from the dash-mpd crate, which implements downloading support for DASH media streams,
  and which now depends on this crate for the struct definitions.

