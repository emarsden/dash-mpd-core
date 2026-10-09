//! Test XSD validation
//
// To run tests while enabling printing to stdout/stderr
//
//    cargo test --test validate --jobs 1 -- --show-output --test-threads=1

pub mod common;
use std::fs;
use std::io;
use std::io::prelude::*;
use fs::File;
use std::path::PathBuf;
use std::process::Command;
use dash_mpd_core::parse;
use common::{setup_logging, curl};
use uppsala::XsdValidator;


// Validate the MPD manifests against the XML schema (XSD) included in the DASH MPD specification.
// Here the XSD validation is performed using the uppsala crate.
#[test]
fn test_fixtures_xsd_validity_uppsala() {
    setup_logging();
    let dir = tempfile::Builder::new()
        // .keep(true)
        .prefix("dash-mpd-xsdtest")
        .rand_bytes(5)
        .tempdir()
        .unwrap();
    let xsd_path = &dir.path().join("DASH-MPD.xsd");
    let _xsd_url_v5l_v5 = "https://raw.githubusercontent.com/MPEGGroup/DASHSchema/refs/heads/5th-Ed-AMD1/DASH-MPD.xsd";
    let xsd_url_v6 = "https://raw.githubusercontent.com/MPEGGroup/DASHSchema/refs/heads/6th-Ed/DASH-MPD.xsd";
    curl(xsd_url_v6, xsd_path).unwrap();
    // Several of these MPDs (taken from various sources in the wild) are known to fail validation
    // with the XSD above. For example, they have no profiles attribute on the MPD elements, they
    // are missing a @minBufferTime attribute on the MPD element, or they are using a Label or
    // AudioChannelConfiguration element where it is not allowed.
    let tests = [
        "a2d-tv.mpd",
        "ad-insertion-testcase1.mpd",
        "ad-insertion-testcase6-av1.mpd",
        "ad-insertion-testcase6-av2.mpd",
        "admanager.xml",
        "avod-mediatailor.mpd",
        "dashif-live-atoinf.mpd",
        "dashif-low-latency.mpd",
        "dash-testcases-5b-1-thomson.mpd",
        "dolby-ac4.xml",
        "jurassic-compact-5975.mpd",
        "mediapackage.xml",
        "st-sl.mpd",
        "telenet-mid-ad-rolls.mpd",
        "telestream-binary.xml",
        "orange.xml",
        "vod-aip-unif-streaming.mpd"
    ];
    let mut counter = 0;
    let mut base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base_path.push("tests");
    base_path.push("fixtures");
    for test in tests {
        counter += 1;
        let mut path = base_path.clone();
        path.push(test);
        let xml = fs::read_to_string(&path).unwrap();
        let serialized = r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_owned()
            + "\n"
            + &parse(&xml).unwrap().to_string();
        let mpd_path = dir.path().join(format!("serialized-{counter}.mpd"));
        fs::write(&mpd_path, &serialized).unwrap();
        let serialized_doc = uppsala::parse(&serialized)
            .expect("serialized MPD is invalid XML");
        let schema_xml= fs::read_to_string(&dir.path().join("DASH-MPD.xsd"))
            .expect("reading DASH XSD contents");
        let schema_doc = uppsala::parse(&schema_xml)
            .expect("parsing DASH XSD");
        let validator = XsdValidator::from_schema(&schema_doc)
            .expect("building XSD validator");
        let errors = validator.validate(&serialized_doc);
        if !errors.is_empty() {
            println!("'{test}' => {} error(s):", errors.len());
        }
        for e in &errors {
            println!("  {}", e);
        }
    }
}


// Validate the MPD manifests against the XML schema (XSD) included in the DASH MPD specification.
// Here the XSD validation is performed using the xmllint commandline tool, which is a little more
// verbose than uppsala. The xmllint tool (really, libxml2) as generally packaged does not
// supporting downloading HTTPS targets for validation (for any schemas that are xlinked inside the
// DASH XSD), only supporting HTTP URLs. Though the URLs in the schema descriptions are HTTP links
// (eg. http://www.w3.org/1999/xlink), they redirect to HTTPS. Note that most alternative validators
// are not preferable: Saxon is not free (Saxon-HE has no schema validation support); xmlstarlet has
// the same limitations because it also binds to libxml2; and the Python xmlvalidate library doesn't
// work well (internal errors during validation).
//
// We work around this libxml2 limitation by putting together our own pseudo system catalog of
// schemas and validating against an XSD that imports these using local schemaLocation URLs.
#[test]
fn test_fixtures_xsd_validity_xmllint() {
    setup_logging();
    let dir = tempfile::Builder::new()
        .prefix("dash-mpd-xsdtest")
        .rand_bytes(5)
        .tempdir()
        .unwrap();
    let xsd_url_v6 = "https://raw.githubusercontent.com/MPEGGroup/DASHSchema/refs/heads/6th-Ed/DASH-MPD.xsd";
    let xsd_up_url = "https://raw.githubusercontent.com/MPEGGroup/DASHSchema/refs/heads/6th-Ed/DASH-MPD-UP.xsd";
    curl(xsd_url_v6,
         &dir.path().join("DASH-MPD.xsd")).unwrap();
    curl(xsd_up_url,
         &dir.path().join("DASH-MPD-UP.xsd")).unwrap();
    curl("http://www.w3.org/XML/2008/06/xlink.xsd",
         &dir.path().join("xlink.xsd")).unwrap();
    curl("http://www.w3.org/2001/xml.xsd",
         &dir.path().join("xml.xsd")).unwrap();
    // We need the xmlns:dashif namespace in order to validate MPDs that contain the dashif:laurl
    // element. This is defined in the DASH-IF-CPS.xsd schema, which should be online at
    // https://dashif.org/CPS, but isn't. We have extracted the content from Annex A of
    // https://dashif.org/docs/IOP-Guidelines/DASH-IF-IOP-Part6-v5.0.0.pdf
    let mut xsd_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xsd_path.push("tests");
    xsd_path.push("fixtures");
    xsd_path.push("DASH-IF-CPS");
    xsd_path.set_extension("xsd");
    fs::copy(xsd_path, dir.path().join("DASH-IF-CPS.xsd")).unwrap();

    let multischema_path = dir.path().join("multischema.xsd");
    let mut multischema_out = File::create(multischema_path).expect("creating multischema.xsd");
    multischema_out.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<schema elementFormDefault=\"qualified\" xmlns=\"http://www.w3.org/2001/XMLSchema\">
  <import namespace=\"http://www.w3.org/2001/xml.xsd\" schemaLocation=\"xml.xsd\"/>
  <import namespace=\"http://www.w3.org/1999/xlink\" schemaLocation=\"xlink.xsd\"/>
  <import namespace=\"urn:mpeg:dash:schema:mpd:2011\" schemaLocation=\"DASH-MPD.xsd\"/>
  <import namespace=\"xmlns:dashif\" schemaLocation=\"DASH-IF-CPS.xsd\"/>
  <import namespace=\"https://dashif.org/CPS\" schemaLocation=\"DASH-IF-CPS.xsd\"/>
</schema>\n").unwrap();

    // Several of these MPDs (taken from various sources in the wild) are known to fail validation
    // with the XSD above. For example, they have no profiles attribute on the MPD elements, they
    // are missing a @minBufferTime attribute on the MPD element, or they are using a Label or
    // AudioChannelConfiguration element where it is not allowed.
    let tests = [
        "a2d-tv.mpd",
        "ad-insertion-testcase1.mpd",
        "ad-insertion-testcase6-av1.mpd",
        "ad-insertion-testcase6-av2.mpd",
        "admanager.xml",
        "avod-mediatailor.mpd",
        "dashif-live-atoinf.mpd",
        "dashif-low-latency.mpd",
        "dash-testcases-5b-1-thomson.mpd",
        "dolby-ac4.xml",
        "jurassic-compact-5975.mpd",
        "mediapackage.xml",
        "st-sl.mpd",
        "telenet-mid-ad-rolls.mpd",
        "telestream-binary.xml",
        "orange.xml",
        "vod-aip-unif-streaming.mpd"
    ];
    let mut counter = 0;
    let mut base_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base_path.push("tests");
    base_path.push("fixtures");
    for test in tests {
        counter += 1;
        let mut path = base_path.clone();
        path.push(test);
        let xml = fs::read_to_string(&path).unwrap();
        let serialized = r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_owned()
            + "\n"
            + &parse(&xml).unwrap().to_string();
        let mpd_path = dir.path().join(format!("serialized-{counter}.mpd"));
        fs::write(&mpd_path, &serialized).unwrap();
        // Format the MPD manifest for better error messages.
        let xmllint = Command::new("xmllint")
            .current_dir(&dir)
            .arg("--format")
            .arg(&mpd_path)
            .arg("--output")
            .arg(format!("formatted-{counter}.mpd"))
            .output()
            .unwrap();
        assert!(xmllint.status.success(), "Failed to format file {}\nstderr: {}",
                mpd_path.display(),
                String::from_utf8_lossy(&xmllint.stderr));
        let xmllint = Command::new("xmllint")
            .current_dir(&dir)
            .arg("--noout")
            .arg("--schema")
            .arg("multischema.xsd")
            .arg(format!("formatted-{counter}.mpd"))
            .output()
            .unwrap();
        if !xmllint.status.success() {
            eprintln!("== failed serializing test {test} on file {}", mpd_path.display());
            let stderr = String::from_utf8_lossy(&xmllint.stderr)
                .lines()
                .filter(|ln| !ln.contains("Skipping import of schema located at"))
                .collect::<Vec<_>>()
                .join("\n");
            println!("xmllint stderr> {stderr}");
        }
    }
}


// The same XSD schema validity test for a bunch of MPD manifests available online.
#[test]
fn test_urls_xsd_validity_uppsala() {
    setup_logging();
    let dir = tempfile::Builder::new()
        .prefix("dash-mpd-xsdtest")
        .rand_bytes(5)
        .tempdir()
        .unwrap();
    let xsd_path = &dir.path().join("DASH-MPD.xsd");
    let _xsd_url_v5l_v5 = "https://raw.githubusercontent.com/MPEGGroup/DASHSchema/refs/heads/5th-Ed-AMD1/DASH-MPD.xsd";
    let xsd_url_v6 = "https://raw.githubusercontent.com/MPEGGroup/DASHSchema/refs/heads/6th-Ed/DASH-MPD.xsd";
    curl(xsd_url_v6, xsd_path).unwrap();
    let tests = [
        "https://cloudflarestream.com/31c9291ab41fac05471db4e73aa11717/manifest/video.mpd",
        "https://v.redd.it/p5rowtg41iub1/DASHPlaylist.mpd?a=1701104071",
        "https://github.com/bbc/exoplayer-testing-samples/raw/master/app/src/androidTest/assets/streams/files/redGreenVideo/redGreenOnlyVideo.mpd",
        "https://dash.akamaized.net/dash264/TestCases/3a/fraunhofer/aac-lc_stereo_without_video/Sintel/sintel_audio_only_aaclc_stereo_sidx.mpd",
        "http://rdmedia.bbc.co.uk/testcard/vod/manifests/radio-flac-en.mpd",
        "https://dash.akamaized.net/dash264/TestCasesMCA/dolby/3/1/ChID_voices_20_128_ddp.mpd",
        "https://dash.akamaized.net/dash264/TestCasesDolby/2/Living_Room_1080p_20_96k_2997fps.mpd",
        "https://cf-sf-video.wmspanel.com/local/raw/BigBuckBunny_320x180.mp4/manifest.mpd",
        "https://ott.dolby.com/OnDelKits/AC-4/Dolby_AC-4_Online_Delivery_Kit_1.5/Test_Signals/muxed_streams/DASH/Live/MPD/Multi_Codec_720p_2997fps_h264.mpd",
        "https://dash.akamaized.net/dash264/TestCasesMCA/dts/1/Paint_dtsc_testA.mpd",
        "https://dash.akamaized.net/dash264/TestCasesHDR/3a/3/MultiRate.mpd",
        "http://refapp.hbbtv.org/videos/01_llama_drama_2160p_25f75g6sv3/manifest.mpd",
        "https://dash.akamaized.net/dash264/TestCasesVP9/vp9-uhd/sintel-vp9-uhd.mpd",
        "http://ftp.itec.aau.at/datasets/mmsys22/Skateboarding/8sec/vvc/manifest.mpd",
        "http://download.tsi.telecom-paristech.fr/gpac/DASH_CONFORMANCE/TelecomParisTech/mpeg2-simple/mpeg2-simple-mpd.mpd",
        "https://origin.broadpeak.io/bpk-vod/voddemo/default/5min/tearsofsteel/manifest.mpd",
        "https://res.cloudinary.com/demo/video/upload/sp_full_hd/handshake.mpd",
        "https://turtle-tube.appspot.com/t/t2/dash.mpd",
        "https://dash.akamaized.net/akamai/test/bbb_enc/BigBuckBunny_320x180_enc_dash.mpd",
        "https://dash.akamaized.net/dash264/TestCases/1a/sony/SNE_DASH_SD_CASE1A_REVISED.mpd",
        "http://ftp.itec.aau.at/datasets/mmsys13/redbull_4sec.mpd",
        "https://dash.akamaized.net/dash264/TestCasesIOP33/adapatationSetSwitching/2/manifest.mpd",
        "https://res.cloudinary.com/demo-robert/video/upload/sp_16x9_vp9/yourPublicId.mpd",
        "https://storage.googleapis.com/shaka-demo-assets/angel-one/dash.mpd",
        "https://demo.unified-streaming.com/k8s/features/stable/video/tears-of-steel/tears-of-steel.ism/.mpd",
        "https://media.axprod.net/TestVectors/H265/clear_cmaf_1080p_h265/manifest.mpd",
        // moved behind Cloudflare on 2026-02
        // "https://cdn.bitmovin.com/content/assets/playhouse-vr/mpds/105560.mpd",
        // "https://www.content-steering.com/bbb/playlist_steering_cloudfront_https.mpd",
        "https://livesim2.dashif.org/livesim2/segtimeline_1/testpic_2s/Manifest.mpd",
        "https://livesim2.dashif.org/livesim2/ato_inf/testpic_2s/Manifest.mpd",
        "https://akamaibroadcasteruseast.akamaized.net/cmaf/live/657078/akasource/out.mpd",
        "https://content.uplynk.com/playlist/6c526d97954b41deb90fe64328647a71.mpd?ad=bbbads&delay=25",
        "https://rdmedia.bbc.co.uk/testcard/vod/manifests/radio-surround-en.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G6.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G10.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G19.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G24.mpd"
    ];
    let mut counter = 0;
    for test in tests {
        counter += 1;
        let resp = reqwest::blocking::get(test).unwrap();
        let body = resp.text().expect("body invalid");
        let dash_filename = dir.path().join(format!("{counter}-orig.mpd"));
        let mut dash_out = File::create(dash_filename).expect("failed to create file");
        io::copy(&mut body.as_bytes(), &mut dash_out).unwrap();
        let serialized = r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_owned() + "\n" +
            &parse(&body)
            .unwrap_or_else(|_| panic!("parsing input {test}"))
            .to_string();
        let mpd_path = dir.path().join(format!("{counter}-serialized.mpd"));
        fs::write(&mpd_path, &serialized).unwrap();
        let serialized_doc = uppsala::parse(&serialized)
            .expect("serialized MPD is invalid XML");
        let schema_xml= fs::read_to_string(&dir.path().join("DASH-MPD.xsd"))
            .expect("reading DASH XSD contents");
        let schema_doc = uppsala::parse(&schema_xml)
            .expect("parsing DASH XSD");
        let validator = XsdValidator::from_schema(&schema_doc)
            .expect("building XSD validator");
        let errors = validator.validate(&serialized_doc);
        if !errors.is_empty() {
            println!("'{test}' => {} error(s):", errors.len());
        }
        for e in &errors {
            println!("  {}", e);
        }
    }
}


#[test]
fn test_urls_xsd_validity_xmllint() {
    setup_logging();
    let dir = tempfile::Builder::new()
        .prefix("dash-mpd-xsdtest")
        .rand_bytes(5)
        .tempdir()
        .unwrap();
    curl("https://raw.githubusercontent.com/MPEGGroup/DASHSchema/refs/heads/5th-Ed-AMD1/DASH-MPD.xsd",
         &dir.path().join("DASH-MPD.xsd")).unwrap();
    curl("http://www.w3.org/XML/2008/06/xlink.xsd",
         &dir.path().join("xlink.xsd")).unwrap();
    curl("http://www.w3.org/2001/xml.xsd",
         &dir.path().join("xml.xsd")).unwrap();
    // The DASH-IF-CPS.xsd schema should be online at https://dashif.org/CPS, but isn't. We have
    // extracted the content from Annex A of
    // https://dashif.org/docs/IOP-Guidelines/DASH-IF-IOP-Part6-v5.0.0.pdf
    let mut xsd_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xsd_path.push("tests");
    xsd_path.push("fixtures");
    xsd_path.push("DASH-IF-CPS");
    xsd_path.set_extension("xsd");
    fs::copy(xsd_path, dir.path().join("DASH-IF-CPS.xsd")).unwrap();

    let multischema_path = dir.path().join("multischema.xsd");
    let mut multischema_out = File::create(multischema_path).expect("creating multischema.xsd");
    multischema_out.write_all(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<schema elementFormDefault=\"qualified\" xmlns=\"http://www.w3.org/2001/XMLSchema\">
  <import namespace=\"http://www.w3.org/2001/xml.xsd\" schemaLocation=\"xml.xsd\"/>
  <import namespace=\"http://www.w3.org/1999/xlink\" schemaLocation=\"xlink.xsd\"/>
  <import namespace=\"urn:mpeg:dash:schema:mpd:2011\" schemaLocation=\"DASH-MPD.xsd\"/>
  <import namespace=\"xmlns:dashif\" schemaLocation=\"DASH-IF-CPS.xsd\"/>
  <import namespace=\"https://dashif.org/CPS\" schemaLocation=\"DASH-IF-CPS.xsd\"/>
</schema>\n").unwrap();

    let tests = [
        "https://cloudflarestream.com/31c9291ab41fac05471db4e73aa11717/manifest/video.mpd",
        "https://v.redd.it/p5rowtg41iub1/DASHPlaylist.mpd?a=1701104071",
        "https://github.com/bbc/exoplayer-testing-samples/raw/master/app/src/androidTest/assets/streams/files/redGreenVideo/redGreenOnlyVideo.mpd",
        "https://dash.akamaized.net/dash264/TestCases/3a/fraunhofer/aac-lc_stereo_without_video/Sintel/sintel_audio_only_aaclc_stereo_sidx.mpd",
        "http://rdmedia.bbc.co.uk/testcard/vod/manifests/radio-flac-en.mpd",
        "https://dash.akamaized.net/dash264/TestCasesMCA/dolby/3/1/ChID_voices_20_128_ddp.mpd",
        "https://dash.akamaized.net/dash264/TestCasesDolby/2/Living_Room_1080p_20_96k_2997fps.mpd",
        "https://cf-sf-video.wmspanel.com/local/raw/BigBuckBunny_320x180.mp4/manifest.mpd",
        "https://ott.dolby.com/OnDelKits/AC-4/Dolby_AC-4_Online_Delivery_Kit_1.5/Test_Signals/muxed_streams/DASH/Live/MPD/Multi_Codec_720p_2997fps_h264.mpd",
        "https://dash.akamaized.net/dash264/TestCasesMCA/dts/1/Paint_dtsc_testA.mpd",
        "https://dash.akamaized.net/dash264/TestCasesHDR/3a/3/MultiRate.mpd",
        "http://refapp.hbbtv.org/videos/01_llama_drama_2160p_25f75g6sv3/manifest.mpd",
        "https://dash.akamaized.net/dash264/TestCasesVP9/vp9-uhd/sintel-vp9-uhd.mpd",
        "http://ftp.itec.aau.at/datasets/mmsys22/Skateboarding/8sec/vvc/manifest.mpd",
        "http://download.tsi.telecom-paristech.fr/gpac/DASH_CONFORMANCE/TelecomParisTech/mpeg2-simple/mpeg2-simple-mpd.mpd",
        "https://origin.broadpeak.io/bpk-vod/voddemo/default/5min/tearsofsteel/manifest.mpd",
        "https://res.cloudinary.com/demo/video/upload/sp_full_hd/handshake.mpd",
        "https://turtle-tube.appspot.com/t/t2/dash.mpd",
        "https://dash.akamaized.net/akamai/test/bbb_enc/BigBuckBunny_320x180_enc_dash.mpd",
        "https://dash.akamaized.net/dash264/TestCases/1a/sony/SNE_DASH_SD_CASE1A_REVISED.mpd",
        "http://ftp.itec.aau.at/datasets/mmsys13/redbull_4sec.mpd",
        "https://dash.akamaized.net/dash264/TestCasesIOP33/adapatationSetSwitching/2/manifest.mpd",
        "https://res.cloudinary.com/demo-robert/video/upload/sp_16x9_vp9/yourPublicId.mpd",
        "https://storage.googleapis.com/shaka-demo-assets/angel-one/dash.mpd",
        "https://demo.unified-streaming.com/k8s/features/stable/video/tears-of-steel/tears-of-steel.ism/.mpd",
        "https://media.axprod.net/TestVectors/H265/clear_cmaf_1080p_h265/manifest.mpd",
        // moved behind Cloudflare on 2026-02
        // "https://cdn.bitmovin.com/content/assets/playhouse-vr/mpds/105560.mpd",
        // "https://www.content-steering.com/bbb/playlist_steering_cloudfront_https.mpd",
        "https://livesim2.dashif.org/livesim2/segtimeline_1/testpic_2s/Manifest.mpd",
        "https://livesim2.dashif.org/livesim2/ato_inf/testpic_2s/Manifest.mpd",
        "https://akamaibroadcasteruseast.akamaized.net/cmaf/live/657078/akasource/out.mpd",
        "https://content.uplynk.com/playlist/6c526d97954b41deb90fe64328647a71.mpd?ad=bbbads&delay=25",
        "https://rdmedia.bbc.co.uk/testcard/vod/manifests/radio-surround-en.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G6.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G10.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G19.mpd",
        "https://github.com/MPEGGroup/DASHSchema/raw/refs/heads/5th-Ed-AMD1/example_G24.mpd"
    ];
    let mut counter = 0;
    for test in tests {
        counter += 1;
        let resp = reqwest::blocking::get(test).unwrap();
        let body = resp.text().expect("body invalid");
        let dash_filename = dir.path().join(format!("{counter}-orig.mpd"));
        let mut dash_out = File::create(dash_filename).expect("failed to create file");
        io::copy(&mut body.as_bytes(), &mut dash_out).unwrap();
        let serialized = r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_owned() + "\n" +
            &parse(&body)
            .unwrap_or_else(|_| panic!("parsing input {test}"))
            .to_string();
        let mpd_path = dir.path().join(format!("{counter}-serialized.mpd"));
        let mpd_formatted = dir.path().join(format!("{counter}-formatted.mpd"));
        fs::write(&mpd_path, &serialized).unwrap();
        // Format the MPD manifest for better error messages.
        let xmllint = Command::new("xmllint")
            .current_dir(&dir)
            .arg("--format")
            .arg(&mpd_path)
            .arg("--output")
            .arg(&mpd_formatted)
            .output()
            .unwrap();
        assert!(xmllint.status.success(), "Failed to format file {}", mpd_path.display());
        let xmllint = Command::new("xmllint")
            .current_dir(&dir)
            .arg("--noout")
            .arg("--schema")
            .arg("multischema.xsd")
            .arg(&mpd_formatted)
            .output()
            .unwrap();
        if !xmllint.status.success() {
            println!("== failed URL serializing test {test} running on {}", mpd_path.display());
            let stderr = String::from_utf8_lossy(&xmllint.stderr)
                .lines()
                .filter(|ln| !ln.contains("Skipping import of schema located at"))
                .collect::<Vec<_>>()
                .join("\n");
            println!("xmllint stderr> {stderr}");
        }
    }
}
