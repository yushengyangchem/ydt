#[test]
fn parses_english_word_result() {
    let html = r#"
    <div class="trans-container">
      <div class="per-phone">
        <span>英</span><span class="phonetic">/həˈləʊ/</span>
      </div>
    </div>
    <div class="trans-container">
      <li class="word-exp">
        <span class="pos">int.</span>
        <span class="trans">你好</span>
      </li>
    </div>
    "#;

    let output = ydt::parse_translation_from_html("hello", html)
        .expect("expected english translation to parse");
    assert_eq!(output, "英 /həˈləʊ/\nint.: 你好");
}

#[test]
fn parses_chinese_word_result() {
    let html = r#"
    <li class="word-exp-ce mcols-layout">
      <a class="point">study</a>
    </li>
    <li class="word-exp-ce mcols-layout">
      <a class="point">learn</a>
    </li>
    "#;

    let output = ydt::parse_translation_from_html("学习", html)
        .expect("expected chinese translation to parse");
    assert_eq!(output, "study\nlearn");
}
