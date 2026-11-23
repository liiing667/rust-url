use form_urlencoded::{parse, parse_chunks, Parser};

/// Feed `body` to a stateful [`Parser`] one byte at a time, flushing every
/// complete pair as soon as its `&` separator (or the end of the body) arrives.
fn parse_in_chunks(body: &[u8], chunk_size: usize) -> Vec<(String, String)> {
    parse_chunks(body.chunks(chunk_size.max(1)))
}

fn parse_whole(body: &[u8]) -> Vec<(String, String)> {
    parse(body)
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

/// For every chunk size, chunked parsing must produce exactly the same
/// multi-value field set as parsing the whole body at once.
fn assert_chunking_invariant(body: &[u8]) {
    let whole = parse_whole(body);
    for chunk_size in 1..=body.len().max(1) {
        assert_eq!(
            parse_in_chunks(body, chunk_size),
            whole,
            "chunk size {} changed the parsed pairs of {:?}",
            chunk_size,
            String::from_utf8_lossy(body),
        );
    }
}

#[test]
fn chunking_matches_whole_input_for_every_boundary() {
    let bodies: &[&[u8]] = &[
        // Repeated keys, a plus, percent signs and an empty value.
        b"foo=1&foo=2&bar=a+b&baz=%E2%9C%93&empty=",
        // A percent-encoded sequence split in all possible ways.
        b"k=%41&k=%42",
        // A literal percent sign that is not part of an escape.
        b"ratio=100%25&half=50%",
        // Key-only pair and consecutive / leading / trailing separators.
        b"&key-only&a=1&&b=2&",
        // Equals signs inside values are preserved.
        b"expr=a%3Db%3Dc&x==",
        // An invalid UTF-8 byte gets the replacement character.
        b"bad=%FF&ok=fine",
        // A multi-byte UTF-8 character with plus signs around it.
        b"text=hello+%E2%9C%93+world",
    ];
    for body in bodies {
        assert_chunking_invariant(body);
    }
}

#[test]
fn repeated_fields_keep_every_value() {
    let body = b"user=alice&user=bob&user=carol";
    for chunk_size in 1..=body.len() {
        let pairs = parse_in_chunks(body, chunk_size);
        assert_eq!(
            pairs,
            vec![
                ("user".to_owned(), "alice".to_owned()),
                ("user".to_owned(), "bob".to_owned()),
                ("user".to_owned(), "carol".to_owned()),
            ],
        );
    }
}

#[test]
fn plus_and_percent_split_across_chunks() {
    // `a+b` (plus must become a space) and `%41` (must decode to `A`) with
    // the split landing inside the `+` byte and inside the escape sequence.
    let body = b"x=a+b&y=%41";
    assert_eq!(
        parse_in_chunks(body, 1),
        vec![
            ("x".to_owned(), "a b".to_owned()),
            ("y".to_owned(), "A".to_owned()),
        ],
    );

    // A multi-byte UTF-8 escape split between every byte.
    let body = b"tick=%E2%9C%93";
    let mut parser = Parser::new();
    let mut seen = Vec::new();
    for chunk in [&b"tick=%E2"[..], b"%9C", b"%93"] {
        seen.extend(parser.parse_next(chunk));
    }
    seen.extend(parser.finish());
    assert_eq!(seen, vec![("tick".to_owned(), "✓".to_owned())]);
}

#[test]
fn empty_values_and_separators() {
    let body = b"a=&b&=c&&d=";
    assert_chunking_invariant(body);
    assert_eq!(
        parse_whole(body),
        vec![
            ("a".to_owned(), String::new()),
            ("b".to_owned(), String::new()),
            (String::new(), "c".to_owned()),
            ("d".to_owned(), String::new()),
        ],
    );

    // A body that ends in a separator has no final pair.
    let mut parser = Parser::new();
    assert_eq!(
        parser.parse_next(b"a=1&"),
        vec![("a".to_owned(), "1".to_owned())]
    );
    assert!(parser.finish().is_empty());

    // Reusing a parser after `finish` starts a fresh form.
    assert_eq!(
        parser.parse_next(b"z=9"),
        Vec::<(String, String)>::new()
    );
    assert_eq!(
        parser.finish(),
        vec![("z".to_owned(), "9".to_owned())]
    );
}

#[test]
fn pairs_are_emitted_as_soon_as_a_separator_arrives() {
    let mut parser = Parser::new();
    assert!(parser.parse_next(b"a=1").is_empty());
    assert_eq!(
        parser.parse_next(b"&b=2&"),
        vec![
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "2".to_owned()),
        ],
    );
    assert!(parser.finish().is_empty());
}

#[test]
fn empty_input_yields_no_pairs() {
    let mut parser = Parser::new();
    assert!(parser.parse_next(b"").is_empty());
    assert!(parser.finish().is_empty());
}

#[test]
fn same_form_same_authorization_view_regardless_of_chunking() {
    // Simulate an authorization view built from a parsed login form: the
    // consumer looks up each repeated field and must observe the same values
    // no matter how the body was split into reads.
    let body = b"username=alice&token=abc+def&token=ghi&remember=";

    let authorize =
        |pairs: &[(String, String)]| -> Vec<(String, String)> {
            pairs
                .iter()
                .filter(|(k, _)| matches!(k.as_str(), "username" | "token" | "remember"))
                .cloned()
                .collect()
        };

    let whole = authorize(&parse_whole(body));
    for chunk_size in 1..=body.len() {
        let chunked = authorize(&parse_in_chunks(body, chunk_size));
        assert_eq!(chunked, whole);
    }

    assert_eq!(
        whole,
        vec![
            ("username".to_owned(), "alice".to_owned()),
            ("token".to_owned(), "abc def".to_owned()),
            ("token".to_owned(), "ghi".to_owned()),
            ("remember".to_owned(), String::new()),
        ],
    );
}
