// Comprehensive benchmarks for string operations
// Tests performance of str_cat, str_join, escape/unescape optimizations
#![cfg(bench)]

use abseil::{
    absl_strings::cord::Cord,
    str_cat,
};

// Import test crate conditionally for benches
#[cfg(test)]
use test::{black_box, Bencher};

// ========== Test Data ==========

fn small_strings() -> Vec<&'static str> {
    vec!["Hello", "world", "test", "bench", "data"]
}

fn medium_strings() -> Vec<&'static str> {
    vec![
        "The quick brown fox",
        "jumps over the lazy",
        "dog and then runs",
        "away to find more",
        "adventures in the",
        "wild and wonderful",
        "world of programming",
        "and testing performance",
    ]
}

fn large_strings() -> Vec<String> {
    (0..100)
        .map(|i| format!("String number {} with some content ", i))
        .collect()
}

fn html_input_small() -> String {
    "<div>Hello & welcome</div>".to_string()
}

fn html_input_medium() -> String {
    "<div><p>Hello & welcome to <b>our site</b></p><a href=\"test\">Link</a></div>".to_string()
}

fn html_input_large() -> String {
    (0..100).map(|i| {
        format!("<div class=\"item{}\"><p>Content {} & \"more\"</p><a href='#link{}'>Link</a></div>", i, i, i)
    }).collect::<Vec<_>>().join("")
}

fn url_input_small() -> String {
    "hello world".to_string()
}

fn url_input_medium() -> String {
    "https://example.com/path with spaces/data?query=value & key=data".to_string()
}

fn url_input_large() -> String {
    (0..100)
        .map(|i| format!("path/segment {}/query?value={}&key={}", i, i, i))
        .collect::<Vec<_>>()
        .join("/")
}

fn c_string_input() -> String {
    "Hello\nWorld\rTest\t\"Quote\" \'Single\' \\Backslash\0Null".to_string()
}

// ========== str_cat! macro benchmarks ==========

#[bench]
fn bench_str_cat_empty(b: &mut Bencher) {
    b.iter(|| {
        let result: String = str_cat!();
        black_box(result);
    });
}

#[bench]
fn bench_str_cat_single_string(b: &mut Bencher) {
    let s = "Hello world";
    b.iter(|| {
        let result = str_cat!(black_box(s));
        black_box(result);
    });
}

#[bench]
fn bench_str_cat_two_strings(b: &mut Bencher) {
    let strings = small_strings();
    b.iter(|| {
        let result = str_cat!(black_box(strings[0]), black_box(strings[1]));
        black_box(result);
    });
}

#[bench]
fn bench_str_cat_five_strings(b: &mut Bencher) {
    let strings = small_strings();
    b.iter(|| {
        let result = str_cat!(
            black_box(strings[0]),
            black_box(strings[1]),
            black_box(strings[2]),
            black_box(strings[3]),
            black_box(strings[4])
        );
        black_box(result);
    });
}

#[bench]
fn bench_str_cat_mixed_types(b: &mut Bencher) {
    b.iter(|| {
        let result = str_cat!(
            "Value: ",
            black_box(42),
            ", ",
            black_box(3.14),
            ", ",
            black_box(true)
        );
        black_box(result);
    });
}

#[bench]
fn bench_str_cat_ten_strings(b: &mut Bencher) {
    let strings = medium_strings();
    b.iter(|| {
        let result = str_cat!(
            black_box(strings[0]),
            black_box(strings[1]),
            black_box(strings[2]),
            black_box(strings[3]),
            black_box(strings[4]),
            black_box(strings[5]),
            black_box(strings[6]),
            black_box(strings[7])
        );
        black_box(result);
    });
}

// ========== StrCat builder benchmarks ==========

#[bench]
fn bench_strcat_builder_small(b: &mut Bencher) {
    let strings = small_strings();
    b.iter(|| {
        let result = StrCat::new()
            .append(black_box(strings[0]))
            .append(black_box(strings[1]))
            .append(black_box(strings[2]))
            .append(black_box(strings[3]))
            .append(black_box(strings[4]))
            .build();
        black_box(result);
    });
}

#[bench]
fn bench_strcat_builder_medium(b: &mut Bencher) {
    let strings = medium_strings();
    b.iter(|| {
        let mut builder = StrCat::new();
        for s in strings.iter() {
            builder = builder.append(black_box(s));
        }
        let result = builder.build();
        black_box(result);
    });
}

#[bench]
fn bench_strcat_builder_large(b: &mut Bencher) {
    let strings = large_strings();
    b.iter(|| {
        let mut builder = StrCat::new();
        for s in strings.iter() {
            builder = builder.append(black_box(s));
        }
        let result = builder.build();
        black_box(result);
    });
}

// ========== str_join benchmarks ==========

#[bench]
fn bench_str_join_empty(b: &mut Bencher) {
    let empty: Vec<&str> = vec![];
    b.iter(|| {
        let result = str_cat::str_join(", ", black_box(&empty));
        black_box(result);
    });
}

#[bench]
fn bench_str_join_small_3_items(b: &mut Bencher) {
    let items = vec!["a", "b", "c"];
    b.iter(|| {
        let result = str_cat::str_join(", ", black_box(&items));
        black_box(result);
    });
}

#[bench]
fn bench_str_join_small_10_items(b: &mut Bencher) {
    let items = (0..10).map(|i| i.to_string()).collect::<Vec<_>>();
    b.iter(|| {
        let result = str_cat::str_join(", ", black_box(&items));
        black_box(result);
    });
}

#[bench]
fn bench_str_join_medium_50_items(b: &mut Bencher) {
    let items = (0..50).map(|i| i.to_string()).collect::<Vec<_>>();
    b.iter(|| {
        let result = str_cat::str_join(", ", black_box(&items));
        black_box(result);
    });
}

#[bench]
fn bench_str_join_large_100_items(b: &mut Bencher) {
    let items = (0..100).map(|i| i.to_string()).collect::<Vec<_>>();
    b.iter(|| {
        let result = str_cat::str_join(", ", black_box(&items));
        black_box(result);
    });
}

#[bench]
fn bench_str_join_empty_delimiter(b: &mut Bencher) {
    let items = vec!["a", "b", "c", "d", "e"];
    b.iter(|| {
        let result = str_cat::str_join("", black_box(&items));
        black_box(result);
    });
}

#[bench]
fn bench_str_join_long_delimiter(b: &mut Bencher) {
    let items = vec!["a", "b", "c"];
    b.iter(|| {
        let result = str_cat::str_join(" | ", black_box(&items));
        black_box(result);
    });
}

// ========== escape_html benchmarks ==========

#[bench]
fn bench_escape_html_small(b: &mut Bencher) {
    let input = html_input_small();
    b.iter(|| {
        let result = escaping::escape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_html_medium(b: &mut Bencher) {
    let input = html_input_medium();
    b.iter(|| {
        let result = escaping::escape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_html_large(b: &mut Bencher) {
    let input = html_input_large();
    b.iter(|| {
        let result = escaping::escape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_html_no_special_chars(b: &mut Bencher) {
    let input = "Hello world this is a plain string with no special characters".to_string();
    b.iter(|| {
        let result = escaping::escape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_html_all_special_chars(b: &mut Bencher) {
    let input = "<>&\"'".repeat(100);
    b.iter(|| {
        let result = escaping::escape_html(black_box(&input));
        black_box(result);
    });
}

// ========== unescape_html benchmarks ==========

#[bench]
fn bench_unescape_html_small(b: &mut Bencher) {
    let input = escaping::escape_html(&html_input_small());
    b.iter(|| {
        let result = escaping::unescape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_unescape_html_medium(b: &mut Bencher) {
    let input = escaping::escape_html(&html_input_medium());
    b.iter(|| {
        let result = escaping::unescape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_unescape_html_large(b: &mut Bencher) {
    let input = escaping::escape_html(&html_input_large());
    b.iter(|| {
        let result = escaping::unescape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_unescape_html_no_entities(b: &mut Bencher) {
    let input = "Hello world this is a plain string with no entities".to_string();
    b.iter(|| {
        let result = escaping::unescape_html(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_unescape_html_only_entities(b: &mut Bencher) {
    let input = "&lt;&gt;&amp;&quot;&apos;".repeat(100);
    b.iter(|| {
        let result = escaping::unescape_html(black_box(&input));
        black_box(result);
    });
}

// ========== escape_url benchmarks ==========

#[bench]
fn bench_escape_url_small(b: &mut Bencher) {
    let input = url_input_small();
    b.iter(|| {
        let result = escaping::escape_url(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_url_medium(b: &mut Bencher) {
    let input = url_input_medium();
    b.iter(|| {
        let result = escaping::escape_url(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_url_large(b: &mut Bencher) {
    let input = url_input_large();
    b.iter(|| {
        let result = escaping::escape_url(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_url_no_special_chars(b: &mut Bencher) {
    let input = "HelloWorld-Test_123".to_string();
    b.iter(|| {
        let result = escaping::escape_url(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_url_all_special_chars(b: &mut Bencher) {
    let input = "a/b?c=d&e=f".repeat(50);
    b.iter(|| {
        let result = escaping::escape_url(black_box(&input));
        black_box(result);
    });
}

// ========== unescape_url benchmarks ==========

#[bench]
fn bench_unescape_url_small(b: &mut Bencher) {
    let input = escaping::escape_url(&url_input_small());
    b.iter(|| {
        let result = escaping::unescape_url(black_box(&input)).unwrap();
        black_box(result);
    });
}

#[bench]
fn bench_unescape_url_medium(b: &mut Bencher) {
    let input = escaping::escape_url(&url_input_medium());
    b.iter(|| {
        let result = escaping::unescape_url(black_box(&input)).unwrap();
        black_box(result);
    });
}

#[bench]
fn bench_unescape_url_large(b: &mut Bencher) {
    let input = escaping::escape_url(&url_input_large());
    b.iter(|| {
        let result = escaping::unescape_url(black_box(&input)).unwrap();
        black_box(result);
    });
}

#[bench]
fn bench_unescape_url_no_encoding(b: &mut Bencher) {
    let input = "HelloWorld-Test_123".to_string();
    b.iter(|| {
        let result = escaping::unescape_url(black_box(&input)).unwrap();
        black_box(result);
    });
}

#[bench]
fn bench_unescape_url_plus_to_space(b: &mut Bencher) {
    let input = "hello+world+test+string".repeat(20);
    b.iter(|| {
        let result = escaping::unescape_url(black_box(&input)).unwrap();
        black_box(result);
    });
}

// ========== escape_c benchmarks ==========

#[bench]
fn bench_escape_c_small(b: &mut Bencher) {
    let input = c_string_input();
    b.iter(|| {
        let result = escaping::escape_c(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_c_newlines(b: &mut Bencher) {
    let input = "line1\nline2\nline3\n".repeat(100);
    b.iter(|| {
        let result = escaping::escape_c(black_box(&input));
        black_box(result);
    });
}

#[bench]
fn bench_escape_c_no_special_chars(b: &mut Bencher) {
    let input = "Hello world this is a plain string".to_string();
    b.iter(|| {
        let result = escaping::escape_c(black_box(&input));
        black_box(result);
    });
}

// ========== unescape_c benchmarks ==========

#[bench]
fn bench_unescape_c_small(b: &mut Bencher) {
    let input = escaping::escape_c(&c_string_input());
    b.iter(|| {
        let result = escaping::unescape_c(black_box(&input)).unwrap();
        black_box(result);
    });
}

#[bench]
fn bench_unescape_c_newlines(b: &mut Bencher) {
    let input = "line1\\nline2\\nline3\\n".repeat(100);
    b.iter(|| {
        let result = escaping::unescape_c(black_box(&input)).unwrap();
        black_box(result);
    });
}

#[bench]
fn bench_unescape_c_no_escape_sequences(b: &mut Bencher) {
    let input = "Hello world this is a plain string".to_string();
    b.iter(|| {
        let result = escaping::unescape_c(black_box(&input)).unwrap();
        black_box(result);
    });
}

// ========== Roundtrip benchmarks ==========

#[bench]
fn bench_html_roundtrip_small(b: &mut Bencher) {
    let input = html_input_small();
    b.iter(|| {
        let escaped = escaping::escape_html(black_box(&input));
        let unescaped = escaping::unescape_html(&escaped);
        black_box(unescaped);
    });
}

#[bench]
fn bench_html_roundtrip_medium(b: &mut Bencher) {
    let input = html_input_medium();
    b.iter(|| {
        let escaped = escaping::escape_html(black_box(&input));
        let unescaped = escaping::unescape_html(&escaped);
        black_box(unescaped);
    });
}

#[bench]
fn bench_url_roundtrip_small(b: &mut Bencher) {
    let input = url_input_small();
    b.iter(|| {
        let escaped = escaping::escape_url(black_box(&input));
        let unescaped = escaping::unescape_url(&escaped).unwrap();
        black_box(unescaped);
    });
}

#[bench]
fn bench_url_roundtrip_medium(b: &mut Bencher) {
    let input = url_input_medium();
    b.iter(|| {
        let escaped = escaping::escape_url(black_box(&input));
        let unescaped = escaping::unescape_url(&escaped).unwrap();
        black_box(unescaped);
    });
}

#[bench]
fn bench_c_roundtrip(b: &mut Bencher) {
    let input = c_string_input();
    b.iter(|| {
        let escaped = escaping::escape_c(black_box(&input));
        let unescaped = escaping::unescape_c(&escaped).unwrap();
        black_box(unescaped);
    });
}
