#![allow(deprecated)]

use assert_matches::assert_matches;

/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_with_length_check() {
    let config = idna::Config::default()
        .verify_dns_length(true)
        .check_hyphens(true)
        .use_std3_ascii_rules(true);

    assert!(config.to_ascii("xn--").is_err());
    assert!(config.to_ascii("xn---").is_err());
    assert!(config.to_ascii("xn-----").is_err());
    assert!(config.to_ascii("xn--.").is_err());
    assert!(config.to_ascii("xn--...").is_err());
    assert!(config.to_ascii(".xn--").is_err());
    assert!(config.to_ascii("...xn--").is_err());
    assert!(config.to_ascii("xn--.xn--").is_err());
    assert!(config.to_ascii("xn--.example.org").is_err());
}

/// https://github.com/servo/rust-url/issues/373
#[test]
fn test_punycode_prefix_without_length_check() {
    let config = idna::Config::default()
        .verify_dns_length(false)
        .check_hyphens(true)
        .use_std3_ascii_rules(true);

    assert!(config.to_ascii("xn--").is_err());
    assert!(config.to_ascii("xn---").is_err());
    assert!(config.to_ascii("xn-----").is_err());
    assert!(config.to_ascii("xn--.").is_err());
    assert!(config.to_ascii("xn--...").is_err());
    assert!(config.to_ascii(".xn--").is_err());
    assert!(config.to_ascii("...xn--").is_err());
    assert!(config.to_ascii("xn--.xn--").is_err());
    assert!(config.to_ascii("xn--.example.org").is_err());
}

#[test]
fn test_punycode_invalid_encoding() {
    let config = idna::Config::default();
    assert!(config.to_ascii("xn--55555577").is_err());
}

// http://www.unicode.org/reports/tr46/#Table_Example_Processing
#[test]
fn test_examples() {
    let mut codec = idna::Idna::default();
    let mut out = String::new();

    assert_matches!(codec.to_unicode("Bloß.de", &mut out), Ok(()));
    assert_eq!(out, "bloß.de");

    out.clear();
    assert_matches!(codec.to_unicode("xn--blo-7ka.de", &mut out), Ok(()));
    assert_eq!(out, "bloß.de");

    out.clear();
    assert_matches!(codec.to_unicode("u\u{308}.com", &mut out), Ok(()));
    assert_eq!(out, "ü.com");

    out.clear();
    assert_matches!(codec.to_unicode("xn--tda.com", &mut out), Ok(()));
    assert_eq!(out, "ü.com");

    out.clear();
    assert_matches!(codec.to_unicode("xn--u-ccb.com", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("a⒈com", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("xn--a-ecp.ru", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("xn--0.pt", &mut out), Err(_));

    out.clear();
    assert_matches!(codec.to_unicode("日本語。ＪＰ", &mut out), Ok(()));
    assert_eq!(out, "日本語.jp");

    out.clear();
    assert_matches!(codec.to_unicode("☕.us", &mut out), Ok(()));
    assert_eq!(out, "☕.us");
}

#[test]
fn test_v5() {
    let config = idna::Config::default()
        .verify_dns_length(true)
        .use_std3_ascii_rules(true)
        .check_hyphens(true);

    // IdnaTest:784 蔏｡𑰺
    assert!(config.to_ascii("\u{11C3A}").is_err());
    assert!(config.to_ascii("\u{850f}.\u{11C3A}").is_err());
    assert!(config.to_ascii("\u{850f}\u{ff61}\u{11C3A}").is_err());
}

#[test]
fn test_v8_bidi_rules() {
    let config = idna::Config::default()
        .verify_dns_length(true)
        .use_std3_ascii_rules(true)
        .check_hyphens(true);

    assert_eq!(config.to_ascii("abc").unwrap(), "abc");
    assert_eq!(config.to_ascii("123").unwrap(), "123");
    assert_eq!(config.to_ascii("אבּג").unwrap(), "xn--kdb3bdf");
    assert_eq!(config.to_ascii("ابج").unwrap(), "xn--mgbcm");
    assert_eq!(config.to_ascii("abc.ابج").unwrap(), "abc.xn--mgbcm");
    assert_eq!(config.to_ascii("אבּג.ابج").unwrap(), "xn--kdb3bdf.xn--mgbcm");

    // Bidi domain names cannot start with digits
    assert!(config.to_ascii("0a.\u{05D0}").is_err());
    assert!(config.to_ascii("0à.\u{05D0}").is_err());

    // Bidi chars may be punycode-encoded
    assert!(config.to_ascii("xn--0ca24w").is_err());
}

#[test]
fn emoji_domains() {
    // HOT BEVERAGE is allowed here...
    let config = idna::Config::default()
        .verify_dns_length(true)
        .use_std3_ascii_rules(true)
        .check_hyphens(true);
    assert_eq!(config.to_ascii("☕.com").unwrap(), "xn--53h.com");
}

#[test]
fn unicode_before_delimiter() {
    let config = idna::Config::default();
    assert!(config.to_ascii("xn--f\u{34a}-PTP").is_err());
}

#[test]
fn test_invalid_utf8_strict_consumer_rejects() {
    use idna::AsciiDenyList;

    // Bytes that are not well-formed UTF-8 (e.g. the result of
    // percent-decoding a host like "http://%FF.com/") must be a hard error
    // for the strict consumer, not silently replaced with U+FFFD.
    assert!(idna::domain_to_ascii_cow(b"\xFF", AsciiDenyList::URL).is_err());
    assert!(idna::domain_to_ascii_cow(b"\xFF.com", AsciiDenyList::URL).is_err());

    // Interleaved good and bad encodings: one bad label anywhere dooms the
    // whole domain, no matter where it appears.
    assert!(idna::domain_to_ascii_cow(b"valid.\xFF.de", AsciiDenyList::URL).is_err());
    assert!(idna::domain_to_ascii_cow(b"\xFF.valid.de", AsciiDenyList::URL).is_err());
    assert!(idna::domain_to_ascii_cow(
        "münchen.\u{FFFD}.de".as_bytes(),
        AsciiDenyList::URL
    )
    .is_err());

    // Valid Unicode keeps working and is converted to Punycode.
    assert_eq!(
        idna::domain_to_ascii_cow("münchen.de".as_bytes(), AsciiDenyList::URL)
            .unwrap()
            .as_ref(),
        "xn--mnchen-3ya.de"
    );
}

#[test]
fn test_replacement_character_does_not_fool_policy() {
    // The lenient consumer marks errors with U+FFFD *and* flags them in the
    // result, so the output cannot be mistaken for a clean value.
    let (marked, result) = idna::domain_to_unicode("exa\u{FFFD}mple.de");
    assert!(result.is_err());
    assert!(marked.contains('\u{FFFD}'));

    // The marked output must not be usable as a policy value: feeding it to
    // the strict consumer is an error, since U+FFFD is disallowed.
    assert!(idna::domain_to_ascii(&marked).is_err());
    assert!(idna::domain_to_ascii("exa\u{FFFD}mple.de").is_err());

    // Valid Unicode still round-trips through both directions.
    let (unicode, result) = idna::domain_to_unicode("xn--mnchen-3ya.de");
    assert!(result.is_ok());
    assert_eq!(unicode, "münchen.de");
    assert_eq!(
        idna::domain_to_ascii("münchen.de").unwrap(),
        "xn--mnchen-3ya.de"
    );
}

#[test]
fn test_failed_decode_leaves_no_decoy_in_output_buffer() {
    // The deprecated API writes into a caller-provided buffer. A failed call
    // reports `Err` and does not leave a partial, seemingly usable value in
    // the buffer...
    let mut codec = idna::Idna::default();
    let mut out = String::new();
    assert!(codec.to_ascii("exa\u{FFFD}mple.de", &mut out).is_err());
    assert!(out.is_empty());

    // ...and a retry with valid input into the same buffer is unaffected by
    // the previous failure.
    assert!(codec.to_ascii("example.de", &mut out).is_ok());
    assert_eq!(out, "example.de");
}
