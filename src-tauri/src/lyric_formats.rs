//! Safe TTML interchange. The original XML is retained for unsupported annotations.
use crate::lyrics::{validate, Document, Line, Word};
use anyhow::{ensure, Context, Result};
use roxmltree::Node;
use serde_json::json;

fn time(value: &str, frame: f64, tick: f64) -> Result<i64> {
    let value = value.trim();
    let seconds = if let Some(v) = value.strip_suffix("ms") {
        v.parse::<f64>()? / 1000.
    } else if let Some(v) = value.strip_suffix('h') {
        v.parse::<f64>()? * 3600.
    } else if let Some(v) = value.strip_suffix('m') {
        v.parse::<f64>()? * 60.
    } else if let Some(v) = value.strip_suffix('s') {
        v.parse::<f64>()?
    } else if let Some(v) = value.strip_suffix('f') {
        v.parse::<f64>()? / frame
    } else if let Some(v) = value.strip_suffix('t') {
        v.parse::<f64>()? / tick
    } else {
        let parts = value
            .split(':')
            .map(str::parse::<f64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(parts.len() <= 3, "Unsupported TTML clock format");
        parts.into_iter().fold(0., |a, b| a * 60. + b)
    };
    ensure!(
        seconds.is_finite() && (0. ..=86400.).contains(&seconds),
        "Invalid TTML timestamp"
    );
    Ok((seconds * 1000.).round() as i64)
}
fn attr<'a>(node: Node<'a, 'a>, name: &str) -> Option<&'a str> {
    match name {
        "agent" | "role" => node.attribute(("http://www.w3.org/ns/ttml#metadata", name)),
        "id" | "space" | "lang" => node.attribute(("http://www.w3.org/XML/1998/namespace", name)),
        "frameRate" | "tickRate" => node
            .attribute(("http://www.w3.org/ns/ttml#parameter", name))
            .or_else(|| node.attribute(name)),
        _ => node.attribute(name),
    }
}

fn role(node: Node<'_, '_>) -> bool {
    matches!(
        attr(node, "role"),
        Some("x-translation" | "x-roman" | "x-bg")
    )
}
fn text(node: Node<'_, '_>) -> String {
    node.children()
        .map(|n| {
            if n.is_text() {
                let t = n.text().unwrap_or("");
                if t.contains('\n') && t.trim().is_empty() {
                    String::new()
                } else {
                    t.into()
                }
            } else if n.is_element() && !role(n) {
                text(n)
            } else {
                String::new()
            }
        })
        .collect()
}
struct Reader {
    doc: Document,
    frame: f64,
    tick: f64,
    singers: serde_json::Map<String, serde_json::Value>,
    absolute: bool,
}
impl Reader {
    fn walk(
        &mut self,
        node: Node<'_, '_>,
        base: i64,
        parent_end: Option<i64>,
        depth: usize,
    ) -> Result<i64> {
        ensure!(depth < 64, "TTML nesting is too deep");
        if node.tag_name().namespace() != Some("http://www.w3.org/ns/ttml") {
            return Ok(base);
        }
        let base = if self.absolute && attr(node, "begin").is_some() {
            0
        } else {
            base
        };
        let start = base
            + attr(node, "begin")
                .map(|v| time(v, self.frame, self.tick))
                .transpose()?
                .unwrap_or(0);
        let end = if let Some(end) = attr(node, "end") {
            Some(base + time(end, self.frame, self.tick)?)
        } else if let Some(dur) = attr(node, "dur") {
            Some(start + time(dur, self.frame, self.tick)?)
        } else {
            parent_end
        };
        ensure!(
            end.is_none_or(|v| v >= start),
            "TTML cue ends before it starts"
        );
        if node.tag_name().name() == "p" || attr(node, "role") == Some("x-bg") {
            let mut line = Line {
                text: text(node),
                start_ms: start,
                end_ms: end,
                ..Default::default()
            };
            let ids = node
                .ancestors()
                .find_map(|n| attr(n, "agent"))
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if let Some(voice) = node.attribute(("urn:musicos:lyrics:1", "voice")) {
                line.extensions
                    .insert("x-musicos".into(), json!({"voice":voice,"agents":ids}));
            } else if !ids.is_empty() {
                let keys = self
                    .singers
                    .iter()
                    .filter(|(_, v)| v["group"] != true)
                    .map(|(k, _)| k)
                    .collect::<Vec<_>>();
                let voice = if ids.len() > 1
                    || ids.iter().any(|id| {
                        self.singers
                            .get(id)
                            .and_then(|v| v.get("group"))
                            .and_then(|v| v.as_bool())
                            == Some(true)
                    }) {
                    "both"
                } else if keys.get(1).is_some_and(|v| **v == ids[0]) {
                    "second"
                } else if keys.first().is_some_and(|v| **v == ids[0]) {
                    "lead"
                } else {
                    "unassigned"
                };
                line.extensions
                    .insert("x-musicos".into(), json!({"voice":voice,"agents":ids}));
            }
            self.words(node, start, end, &mut line.words, 0)?;
            if !line.words.is_empty() {
                let mut cursor = 0;
                for i in 0..line.words.len() {
                    let spoken = line.words[i].text.trim().to_string();
                    ensure!(!spoken.is_empty(), "TTML contains an empty timed word");
                    let offset = line.text[cursor..]
                        .find(&spoken)
                        .context("TTML timed words do not match the visible line")?;
                    let start = cursor + offset;
                    ensure!(!line.text[cursor..start].chars().any(|c|c.is_alphanumeric()),"This TTML mixes untimed vocal text with word timing; check the source before importing");
                    if i > 0 {
                        line.words[i - 1].text.push_str(&line.text[cursor..start]);
                        line.words[i].text = spoken.clone();
                    } else {
                        line.words[i].text = format!("{}{}", &line.text[cursor..start], spoken);
                    }
                    cursor = start + spoken.len();
                }
                let tail = &line.text[cursor..];
                ensure!(
                    !tail.chars().any(|c| c.is_alphanumeric()),
                    "This TTML contains untimed vocal text after its timed words"
                );
                if let Some(last) = line.words.last_mut() {
                    last.text.push_str(tail);
                }
            }

            if !line.text.trim().is_empty() {
                self.doc.lines.push(line);
            }
            for bg in node
                .descendants()
                .skip(1)
                .filter(|n| n.is_element() && attr(*n, "role") == Some("x-bg"))
            {
                self.walk(bg, start, end, depth + 1)?;
            }
            return Ok(end.unwrap_or(start));
        }
        let sequence = attr(node, "timeContainer") == Some("seq");
        let mut cursor = start;
        let mut last = start;
        for child in node.children().filter(|n| n.is_element()) {
            if role(child) {
                continue;
            }
            let until = self.walk(child, if sequence { cursor } else { start }, end, depth + 1)?;
            cursor = until;
            last = last.max(until);
        }
        Ok(end.unwrap_or(last))
    }
    fn words(
        &self,
        node: Node<'_, '_>,
        base: i64,
        end: Option<i64>,
        out: &mut Vec<Word>,
        depth: usize,
    ) -> Result<()> {
        ensure!(depth < 64, "TTML nesting is too deep");
        for child in node.children() {
            if child.is_text() {
                let value = child.text().unwrap_or("");
                if !value.contains('\n') && value.trim().is_empty() {
                    if let Some(word) = out.last_mut() {
                        word.text.push_str(value);
                    }
                }
                continue;
            }
            if !child.is_element() || role(child) {
                continue;
            }
            let base = if self.absolute && attr(child, "begin").is_some() {
                0
            } else {
                base
            };
            let start = base
                + attr(child, "begin")
                    .map(|v| time(v, self.frame, self.tick))
                    .transpose()?
                    .unwrap_or(0);
            let finish = if let Some(v) = attr(child, "end") {
                Some(base + time(v, self.frame, self.tick)?)
            } else if let Some(v) = attr(child, "dur") {
                Some(start + time(v, self.frame, self.tick)?)
            } else {
                end
            };
            if attr(child, "begin").is_some()
                && !child
                    .descendants()
                    .skip(1)
                    .any(|n| n.is_element() && attr(n, "begin").is_some())
            {
                out.push(Word {
                    text: text(child),
                    start_ms: start,
                    end_ms: finish,
                    ..Default::default()
                });
            } else {
                self.words(child, start, finish, out, depth + 1)?;
            }
        }
        Ok(())
    }
}
pub fn parse_ttml(source: &str) -> Result<Document> {
    ensure!(
        source.len() <= 2_000_000,
        "Choose a lyric file smaller than 2 MB"
    );
    let xml = roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 100_000,
            ..Default::default()
        },
    )
    .context("Invalid TTML document")?;
    let root = xml.root_element();
    ensure!(
        root.tag_name().name() == "tt"
            && root.tag_name().namespace() == Some("http://www.w3.org/ns/ttml"),
        "Choose a TTML lyrics document"
    );
    let mut reader = Reader {
        doc: Document::default(),
        frame: attr(root, "frameRate").unwrap_or("30").parse()?,
        tick: attr(root, "tickRate").unwrap_or("1").parse()?,
        singers: Default::default(),
        absolute: root.namespaces().any(|n| n.uri().contains("lyric-ttml")),
    };
    ensure!(
        reader.frame > 0. && reader.tick > 0.,
        "Invalid TTML clock rate"
    );
    for n in root.descendants().filter(|n| {
        n.is_element()
            && n.tag_name().name() == "agent"
            && n.tag_name().namespace() == Some("http://www.w3.org/ns/ttml#metadata")
    }) {
        if let Some(id) = attr(n, "id") {
            let name = n
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "name")
                .map(text)
                .unwrap_or_else(|| id.into());
            reader.singers.insert(
                id.into(),
                json!({"name":name,"group":attr(n,"type")==Some("group")}),
            );
        }
    }
    reader.doc.metadata.title = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "title")
        .map(text)
        .unwrap_or_default();
    reader.doc.metadata.language = attr(root, "lang").map(str::to_owned);
    let body = root
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "body")
        .context("TTML has no lyric body")?;
    reader.walk(body, 0, None, 0)?;
    reader.doc.lines.sort_by_key(|l| l.start_ms);
    reader.doc.plain = reader
        .doc
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    reader.doc.extensions.insert(
        "x-musicos".into(),
        json!({"singers":reader.singers,"original":{"format":"ttml","text":source}}),
    );
    validate(&reader.doc)?;
    Ok(reader.doc)
}
fn escaped(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn stamp(ms: i64) -> String {
    format!("{}.{:03}s", ms / 1000, ms % 1000)
}
pub fn export_ttml(doc: &Document) -> Result<String> {
    validate(doc)?;
    ensure!(!doc.lines.is_empty(), "TTML export needs timed lines");
    let mut xml=String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<tt xmlns=\"http://www.w3.org/ns/ttml\" xmlns:ttm=\"http://www.w3.org/ns/ttml#metadata\" xmlns:m=\"urn:musicos:lyrics:1\"><head><metadata>");
    xml.push_str(&format!(
        "<ttm:title>{}</ttm:title>",
        escaped(&doc.metadata.title)
    ));
    if let Some(singers) = doc
        .extensions
        .get("x-musicos")
        .and_then(|v| v.get("singers"))
        .and_then(|v| v.as_object())
    {
        for (id, singer) in singers {
            xml.push_str(&format!(
                "<ttm:agent xml:id=\"{}\" type=\"{}\"><ttm:name>{}</ttm:name></ttm:agent>",
                escaped(id),
                if singer["group"] == true {
                    "group"
                } else {
                    "person"
                },
                escaped(singer["name"].as_str().unwrap_or(id))
            ));
        }
    }
    xml.push_str("</metadata></head><body><div>");
    for l in &doc.lines {
        let extension = l.extensions.get("x-musicos");
        let agents = extension
            .and_then(|v| v.get("agents"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let voice = extension
            .and_then(|v| v.get("voice"))
            .and_then(|v| v.as_str())
            .unwrap_or("unassigned");
        xml.push_str(&format!(
            "<p begin=\"{}\"{} ttm:agent=\"{}\" m:voice=\"{}\">",
            stamp((l.start_ms + doc.metadata.offset_ms).max(0)),
            l.end_ms
                .map(|v| format!(" end=\"{}\"", stamp((v + doc.metadata.offset_ms).max(0))))
                .unwrap_or_default(),
            escaped(&agents),
            escaped(voice)
        ));
        if l.words.is_empty() {
            xml.push_str(&escaped(&l.text));
        } else {
            for w in &l.words {
                xml.push_str(&format!(
                    "<span begin=\"{}\"{}>{}</span>",
                    stamp((w.start_ms - l.start_ms).max(0)),
                    w.end_ms
                        .map(|v| format!(" end=\"{}\"", stamp((v - l.start_ms).max(0))))
                        .unwrap_or_default(),
                    escaped(&w.text)
                ));
            }
        }
        xml.push_str("</p>");
    }
    xml.push_str("</div></body></tt>");
    Ok(xml)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_words_and_duets() {
        let doc=parse_ttml(r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><head><metadata><ttm:agent xml:id="v1"><ttm:name>A</ttm:name></ttm:agent><ttm:agent xml:id="v2"><ttm:name>B</ttm:name></ttm:agent></metadata></head><body><div begin="2s"><p begin="1s" end="4s" ttm:agent="v1"><span begin="0s" end="1s">Hello</span></p><p begin="2s" end="5s" ttm:agent="v2">Reply</p></div></body></tt>"#).unwrap();
        assert_eq!(doc.lines[0].start_ms, 3000);
        assert_eq!(doc.lines[0].words[0].end_ms, Some(4000));
        assert_eq!(doc.lines[1].extensions["x-musicos"]["voice"], "second");
        let again = parse_ttml(&export_ttml(&doc).unwrap()).unwrap();
        assert_eq!(again.lines[0].words, doc.lines[0].words);
        assert_eq!(again.lines[1].start_ms, 4000);
    }
    #[test]
    fn external_entities_rejected() {
        assert!(parse_ttml(r#"<!DOCTYPE tt [<!ENTITY x SYSTEM "file:///etc/passwd">]><tt xmlns="http://www.w3.org/ns/ttml"><body><p begin="0s">&x;</p></body></tt>"#).is_err());
    }
}
#[cfg(test)]
mod compatibility_tests {
    use super::*;
    #[test]
    fn lyricsfile_extensions_round_trip() {
        let original="version: '1.0'\nmetadata:\n  title: Test\n  artist: Example\n  x-credit: Someone\nlines:\n- text: Hi\n  start_ms: 100\n  end_ms: 500\n  x-musicos:\n    voice: second\n  x-extra: [one, two]\nx-future:\n  value: true\n";
        let doc = crate::lyrics::parse(original, "lyricsfile").unwrap();
        let again = crate::lyrics::parse(
            &crate::lyrics::export(&doc, "lyricsfile").unwrap(),
            "lyricsfile",
        )
        .unwrap();
        assert_eq!(again, doc);
        assert_eq!(again.lines[0].extensions["x-musicos"]["voice"], "second");
    }
    #[test]
    fn apple_absolute_words_and_spaces() {
        let d=parse_ttml(r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://itunes.apple.com/lyric-ttml-extensions"><body><div begin="2s"><p begin="3s" end="8s"><span begin="3s" end="4s">Hello</span> <span begin="4s" end="5s">world</span></p></div></body></tt>"#).unwrap();
        assert_eq!(d.lines[0].start_ms, 3000);
        assert_eq!(d.lines[0].words.len(), 2);
        assert_eq!(d.lines[0].words[0].text, "Hello ");
        assert_eq!(d.lines[0].words[1].start_ms, 4000);
    }
    #[test]
    fn background_voice_is_a_separate_cue() {
        let d=parse_ttml(r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata"><body><p begin="1s" end="5s">Lead<span ttm:role="x-bg" begin="1s" end="2s">Backing</span><span ttm:role="x-translation">Translation</span></p></body></tt>"#).unwrap();
        assert_eq!(d.lines.len(), 2);
        assert_eq!(d.lines[0].text, "Lead");
        assert_eq!(d.lines[1].text, "Backing");
        assert!(d.extensions["x-musicos"]["original"]["text"]
            .as_str()
            .unwrap()
            .contains("Translation"));
    }
}
