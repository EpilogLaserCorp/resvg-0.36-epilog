//! Tests that the writer escapes every reserved character it emits, so its output stays parsable
//! and survives a round trip unchanged.

use usvg::{NodeKind, TreeParsing, TreeWriting, XmlOptions};

/// Every character the writer escapes, in the form a source SVG attribute must use for it.
const RESERVED: &str = "a&amp;b&lt;c&gt;d&quot;e&apos;f&#9;g&#10;h&#13;i";

/// The same characters as they must appear in the writer's output.
const RESERVED_ESCAPED: &str = "a&amp;b&lt;c&gt;d&quot;e&apos;f&#9;g&#10;h&#13;i";

fn parse(svg: &str) -> usvg::Tree {
    usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap()
}

fn write(svg: &str) -> String {
    parse(svg).to_string(&XmlOptions::default())
}

/// Serializing, reparsing and serializing again must produce identical output. Under-escaping shows
/// up here as either a parse failure or a silently altered value.
fn assert_round_trips(svg: &str) -> String {
    let once = write(svg);
    let twice = write(&once);
    assert_eq!(once, twice, "output changed when reparsed:\n{}", once);
    once
}

fn assert_contains(output: &str, expected: &str, what: &str) {
    assert!(
        output.contains(expected),
        "{} not escaped, expected {:?} in:\n{}",
        what,
        expected,
        output
    );
}

/// A `url(#..)` reference only resolves for ids without `<`, `>` or quotes, so ids are tested with
/// the subset that can actually reach the writer through a reference.
const RESERVED_ID: &str = "a&amp;b&#9;c&#10;d&#13;e";
const RESERVED_ID_ESCAPED: &str = "a&amp;b&#9;c&#10;d&#13;e";

#[test]
fn escapes_ids_and_references() {
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'>
            <clipPath id='{0}'>
                <rect width='10' height='10'/>
            </clipPath>
            <rect clip-path='url(#{0})' width='10' height='10'/>
        </svg>",
        RESERVED_ID
    );

    let output = assert_round_trips(&svg);
    assert_contains(&output, &format!("id=\"{}\"", RESERVED_ID_ESCAPED), "id");
    assert_contains(
        &output,
        &format!("clip-path=\"url(#{})\"", RESERVED_ID_ESCAPED),
        "clip-path reference",
    );
}

/// A clipped rect, for the tests that prefix its ids.
const CLIPPED_RECT: &str = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'>
    <clipPath id='clip'>
        <rect width='10' height='10'/>
    </clipPath>
    <rect clip-path='url(#clip)' width='10' height='10'/>
</svg>";

fn prefixed(prefix: &str) -> String {
    let opt = XmlOptions {
        id_prefix: Some(prefix.to_string()),
        ..XmlOptions::default()
    };
    parse(CLIPPED_RECT).to_string(&opt)
}

#[test]
fn escapes_id_prefix() {
    // Unescaped, the tab, newline and carriage return would be normalized to spaces on reparse,
    // silently breaking the reference.
    let output = prefixed("a&b\tc\nd\re");

    assert_contains(&output, "id=\"a&amp;b&#9;c&#10;d&#13;eclip\"", "id prefix");
    assert_contains(
        &output,
        "clip-path=\"url(#a&amp;b&#9;c&#10;d&#13;eclip)\"",
        "id prefix in reference",
    );
    // The prefixed reference must still resolve, not just parse.
    assert_eq!(
        output,
        write(&output),
        "prefixed output changed when reparsed:\n{}",
        output
    );
}

/// A prefix comes from the caller rather than the parser, so it can also carry quotes. `url(#..)`
/// never resolves an id containing one, so only the written form can be checked here.
#[test]
fn escapes_quotes_in_id_prefix() {
    let output = prefixed("a&b\"c");

    assert_contains(&output, "id=\"a&amp;b&quot;cclip\"", "quoted id prefix");
    assert_contains(
        &output,
        "clip-path=\"url(#a&amp;b&quot;cclip)\"",
        "quoted id prefix in reference",
    );
    // Still well-formed XML, even though the reference no longer resolves.
    parse(&output);
}

#[test]
fn escapes_font_families() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <text x='0' y='50' font-family=\"&apos;a&amp;b&lt;c&gt;d&apos;\">text</text>
    </svg>";

    let output = assert_round_trips(svg);
    assert_contains(
        &output,
        "font-family=\"&quot;a&amp;b&lt;c&gt;d&quot;\"",
        "font-family",
    );
}

#[test]
fn escapes_text_content() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <text x='0' y='50'>a&amp;b&lt;c&gt;d&quot;e&apos;f</text>
    </svg>";

    let output = assert_round_trips(svg);
    assert_contains(&output, "a&amp;b&lt;c&gt;d&quot;e&apos;f", "text content");
}

/// The parser collapses whitespace in text nodes, so reach those branches through a tree built up
/// by hand — the way any library consumer can.
#[test]
fn escapes_whitespace_in_text_content() {
    let tree = parse(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
            <text x='0' y='50'>axbxcxd</text>
        </svg>",
    );

    for node in tree.root.descendants() {
        if let NodeKind::Text(ref mut text) = *node.borrow_mut() {
            for chunk in &mut text.chunks {
                // Same byte length, so the spans' offsets into the chunk stay valid.
                chunk.text = "a\tb\nc\rd".to_string();
            }
        }
    }

    let output = tree.to_string(&XmlOptions::default());
    assert_contains(&output, "a&#9;b&#10;c&#13;d", "text whitespace");
    // Reparsing must not fail; the whitespace itself is collapsed by the parser.
    write(&output);
}

#[test]
fn escapes_fe_image_href() {
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' viewBox='0 0 10 10'>
            <filter id='filter'>
                <feImage xlink:href='#{0}'/>
            </filter>
            <rect id='{0}' width='10' height='10' fill='green'/>
            <rect filter='url(#filter)' width='10' height='10'/>
        </svg>",
        RESERVED
    );

    let output = assert_round_trips(&svg);
    assert_contains(
        &output,
        &format!("xlink:href=\"#{}\"", RESERVED_ESCAPED),
        "feImage href",
    );
}

#[test]
fn escapes_filter_reference_list() {
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'>
            <filter id='{0}'>
                <feGaussianBlur stdDeviation='1'/>
            </filter>
            <rect filter='url(#{0})' width='10' height='10'/>
        </svg>",
        RESERVED
    );

    let output = assert_round_trips(&svg);
    assert_contains(
        &output,
        &format!("filter=\"url(#{})\"", RESERVED_ESCAPED),
        "filter reference",
    );
}

#[test]
fn leaves_non_reserved_characters_alone() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>
        <text x='0' y='50' font-family='Liberation Sans'>é漢🙂</text>
    </svg>";

    let output = assert_round_trips(svg);
    assert_contains(&output, "é漢🙂", "text content");
    assert_contains(&output, "Liberation Sans", "font-family");
}
