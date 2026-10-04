# dash-mpd-core

A Rust library for parsing and serializing a DASH MPD manifest, as used by media streaming services
such as on-demand replay of TV content, video streaming services like YouTube and live media
streaming. Allows both parsing the XML content of a DASH manifest to Rust structs (deserialization) and
programmatically generating an MPD manifest and serializing it to XML.


[![Crates.io](https://img.shields.io/crates/v/dash-mpd-core)](https://crates.io/crates/dash-mpd-core)
[![Released API docs](https://docs.rs/dash-mpd-core/badge.svg)](https://docs.rs/dash-mpd-core/)
[![CI](https://github.com/emarsden/dash-mpd-core/workflows/build/badge.svg)](https://github.com/emarsden/dash-mpd-core/actions/workflows/ci.yml)
[![Dependency status](https://deps.rs/repo/github/emarsden/dash-mpd-core/status.svg)](https://deps.rs/repo/github/emarsden/dash-mpd-core)
[![Recent crates.io downloads](https://img.shields.io/crates/dr/dash-mpd-core?label=crates.io%20recent%20downloads)](https://crates.io/crates/dash-mpd-core)
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)

[DASH](https://en.wikipedia.org/wiki/Dynamic_Adaptive_Streaming_over_HTTP) (dynamic adaptive
streaming over HTTP), also called MPEG-DASH, is a technology used for media streaming over the web,
commonly used for video on demand (VOD) services. The Media Presentation Description (MPD) is a
description of the resources (manifest or “playlist”) forming a streaming service, that a DASH
client uses to determine which assets to request in order to perform adaptive streaming of the
content. DASH MPD manifests can be used both with content encoded as H.264/MPEG and as WebM, and
with file segments using either MPEG-2 Transport Stream (M2TS) container format or fragmented MPEG-4
(also called CFF). There is a good explanation of adaptive bitrate video streaming at
[howvideo.works](https://howvideo.works/#dash).

This library provides a serde-based parser (deserializer) and serializer for the DASH MPD format, as
formally defined in ISO/IEC standard 23009-1:2022 (this is the fifth edition). XML schema files are
[available for no cost from
ISO](https://standards.iso.org/ittf/PubliclyAvailableStandards/MPEG-DASH_schema_files/). The library
also provides non-exhaustive support for certain DASH extensions such as the DVB-DASH and HbbTV
(Hybrid Broadcast Broadband TV) profiles. When MPD files in practical use diverge from the formal
standard(s), this library prefers to interoperate with existing practice.



## Usage

To **parse** (deserialize) the contents of an MPD manifest into Rust structs:

```rust
use std::time::Duration;
use dash_mpd_core::{MPD, parse};

fn main() {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::new(30, 0))
        .build()
        .expect("creating HTTP client");
    let xml = client.get("https://rdmedia.bbc.co.uk/testcard/vod/manifests/avc-ctv-stereo-en.mpd")
        .header("Accept", "application/dash+xml,video/vnd.mpeg.dash.mpd")
        .send()
        .expect("requesting MPD content")
        .text()
        .expect("fetching MPD content");
    let mpd: MPD = parse(&xml)
        .expect("parsing MPD");
    for pi in &mpd.ProgramInformation {
        if let Some(title) = pi.Title {
            println!("Title: {:?}", title.content);
        }
        if let Some(source) = pi.Source {
            println!("Source: {:?}", source.content);
        }
    }
    for p in mpd.periods {
        if let Some(d) = p.duration {
            println!("Contains Period of duration {d:?}");
        }
    }
}
```

See example
[pprint_bbc_adaptive.rs](https://github.com/emarsden/dash-mpd-core/blob/main/examples/pprint_bbc_adaptive.rs)
for more information.


To **generate an MPD manifest programmatically**:

```rust
use dash_mpd_core::{MPD, ProgramInformation, Title};

fn main() {
   let pi = ProgramInformation {
       Title: Some(Title { content: Some("My serialization example".into()) }),
       lang: Some("eng".into()),
       moreInformationURL: Some("https://github.com/emarsden/dash-mpd-core".into()),
       ..Default::default()
   };
   let mpd = MPD {
       mpdtype: Some("static".into()),
       xmlns: Some("urn:mpeg:dash:schema:mpd:2011".into()),
       ProgramInformation: vec!(pi),
       ..Default::default()
   };

   let xml = mpd.to_string();
}
```

See example [serialize.rs](https://github.com/emarsden/dash-mpd-core/blob/main/examples/serialize.rs) for more detail.


## Optional features

The following additive [Cargo
features](https://doc.rust-lang.org/stable/cargo/reference/features.html#the-features-section) can
be enabled:

- `scte35` *(enabled by default)*: enable support for XML elements corresponding to the SCTE-35
  standard for insertion of alternate content (mostly used for dynamic insertion of advertising).

- `warn_ignored_elements`: if this feature is enabled, a warning will be issued when an XML element
  present in the DASH manifest is not deserialized into a Rust struct, while parsing the manifest.
  The default behaviour is to ignore elements for which we have not defined serde deserialization
  instructions. This feature is implemented with the `serde_ignored` crate.


We endeavour to use **semantic versioning** for this crate despite its 0.x version number: a major
change which requires users of the library to change their code (such as a change in an attribute
name or type) will be published in a major release. For a version number `0.y.z`, a major release
implies a change to `y`.



## License

This project is licensed under the MIT license. For more information, see the `LICENSE-MIT` file.

Patches and pull requests are welcome.
