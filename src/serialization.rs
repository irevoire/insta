use serde::de::value::Error as ValueError;
use serde::Serialize;

use crate::content::{Content, ContentSerializer};
use crate::settings::Settings;

pub enum SerializationFormat {
    #[cfg(feature = "csv")]
    Csv,
    #[cfg(feature = "ron")]
    Ron,
    #[cfg(feature = "toml")]
    Toml,
    // #[cfg(feature = "yaml")]
    Yaml,
    // #[cfg(feature = "json")]
    Json,
}

pub enum SnapshotLocation {
    Inline,
    File,
}

pub fn serialize_content(
    mut _content: Content,
    format: SerializationFormat,
    _location: SnapshotLocation,
) -> String {
    _content = Settings::with(|settings| {
        if settings.sort_maps() {
            _content.sort_maps();
        }
        #[cfg(feature = "redactions")]
        {
            for (selector, redaction) in settings.iter_redactions() {
                _content = selector.redact(_content, redaction);
            }
        }
        _content
    });

    match format {
        // #[cfg(feature = "yaml")]
        SerializationFormat::Yaml => {
            let serialized = _content.as_yaml();
            match _location {
                SnapshotLocation::Inline => serialized,
                SnapshotLocation::File => serialized[4..].to_string(),
            }
        }
        // #[cfg(feature = "json")]
        SerializationFormat::Json => serde_json::to_string_pretty(&_content).unwrap(),
        #[cfg(feature = "csv")]
        SerializationFormat::Csv => {
            let mut buf = Vec::with_capacity(128);
            {
                let mut writer = dep_csv::Writer::from_writer(&mut buf);
                // if the top-level content we're serializing is a vector we
                // want to serialize it multiple times once for each item.
                if let Some(content_slice) = _content.as_slice() {
                    for content in content_slice {
                        writer.serialize(content).unwrap();
                    }
                } else {
                    writer.serialize(&_content).unwrap();
                }
                writer.flush().unwrap();
            }
            if buf.ends_with(b"\n") {
                buf.truncate(buf.len() - 1);
            }
            String::from_utf8(buf).unwrap()
        }
        #[cfg(feature = "ron")]
        SerializationFormat::Ron => {
            let mut buf = Vec::new();
            let mut config = dep_ron::ser::PrettyConfig::new();
            config.new_line = "\n".to_string();
            config.indentor = "  ".to_string();
            config.struct_names = true;
            let mut serializer = dep_ron::ser::Serializer::with_options(
                &mut buf,
                Some(config),
                dep_ron::options::Options::default(),
            )
            .unwrap();
            _content.serialize(&mut serializer).unwrap();
            String::from_utf8(buf).unwrap()
        }
        #[cfg(feature = "toml")]
        SerializationFormat::Toml => {
            let mut rv = dep_toml::to_string_pretty(&_content).unwrap();
            if rv.ends_with('\n') {
                rv.truncate(rv.len() - 1);
            }
            rv
        }
    }
}

pub fn serialize_value<S: Serialize>(
    s: &S,
    format: SerializationFormat,
    location: SnapshotLocation,
) -> String {
    let serializer = ContentSerializer::<ValueError>::new();
    let content = Serialize::serialize(s, serializer).unwrap();
    serialize_content(content, format, location)
}

#[cfg(feature = "redactions")]
pub fn serialize_value_redacted<S: Serialize>(
    s: &S,
    redactions: &[(crate::redaction::Selector, crate::redaction::Redaction)],
    format: SerializationFormat,
    location: SnapshotLocation,
) -> String {
    let serializer = ContentSerializer::<ValueError>::new();
    let mut content = Serialize::serialize(s, serializer).unwrap();
    for (selector, redaction) in redactions {
        content = selector.redact(content, redaction);
    }
    serialize_content(content, format, location)
}

#[test]
fn test_yaml_serialization() {
    let yaml = serialize_content(
        Content::Map(vec![
            (
                Content::from("env"),
                Content::Seq(vec![
                    Content::from("ENVIRONMENT"),
                    Content::from("production"),
                ]),
            ),
            (
                Content::from("cmdline"),
                Content::Seq(vec![Content::from("my-tool"), Content::from("run")]),
            ),
        ]),
        SerializationFormat::Yaml,
        SnapshotLocation::File,
    );
    crate::assert_snapshot!(&yaml, @r###"
    env:
      - ENVIRONMENT
      - production
    cmdline:
      - my-tool
      - run
    "###);

    let inline_yaml = serialize_content(
        Content::Map(vec![
            (
                Content::from("env"),
                Content::Seq(vec![
                    Content::from("ENVIRONMENT"),
                    Content::from("production"),
                ]),
            ),
            (
                Content::from("cmdline"),
                Content::Seq(vec![Content::from("my-tool"), Content::from("run")]),
            ),
        ]),
        SerializationFormat::Yaml,
        SnapshotLocation::Inline,
    );
    crate::assert_snapshot!(&inline_yaml, @r###"
    ---
    env:
      - ENVIRONMENT
      - production
    cmdline:
      - my-tool
      - run
    "###);
}
