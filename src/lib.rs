//! `ydt` library API.
//!
//! This crate provides a simple way to fetch and parse translations from Youdao.

use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use reqwest::Url;
use scraper::{Html, Selector};
use std::error::Error;
use std::fmt;
use std::sync::OnceLock;
use std::time::Duration;

const PROJECT_USER_AGENT: &str = concat!(
    "ydt/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/yushengyangchem/ydt)"
);
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
const YOUDAO_RESULT_URL: &str = "https://www.youdao.com/result";

static WORD_EXP_CE_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static POINT_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static TRANS_CONTAINER_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static PHONE_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static SPAN_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static PHONETIC_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static WORD_EXP_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static POS_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();
static TRANS_SELECTOR: OnceLock<Result<Selector, YdtError>> = OnceLock::new();

#[derive(Debug)]
/// Error type returned by `ydt` public APIs.
pub enum YdtError {
    CreateHttpClient(reqwest::Error),
    BuildRequestUrl(url::ParseError),
    FetchTranslation(reqwest::Error),
    HttpStatus(StatusCode),
    ReadResponse(reqwest::Error),
    ParseCssSelector(&'static str),
}

impl fmt::Display for YdtError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateHttpClient(err) => write!(f, "Failed to create HTTP client: {err}"),
            Self::BuildRequestUrl(err) => write!(f, "Failed to build request URL: {err}"),
            Self::FetchTranslation(err) => write!(f, "Failed to fetch translation: {err}"),
            Self::HttpStatus(status) => write!(f, "Request failed with status: {status}"),
            Self::ReadResponse(err) => write!(f, "Failed to read response: {err}"),
            Self::ParseCssSelector(css) => write!(f, "Failed to parse CSS selector: {css}"),
        }
    }
}

impl Error for YdtError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateHttpClient(err) => Some(err),
            Self::BuildRequestUrl(err) => Some(err),
            Self::FetchTranslation(err) => Some(err),
            Self::ReadResponse(err) => Some(err),
            Self::HttpStatus(_) => None,
            Self::ParseCssSelector(_) => None,
        }
    }
}

fn contains_cjk_ideograph(text: &str) -> bool {
    text.chars().any(|ch| {
        ('\u{3400}'..='\u{4DBF}').contains(&ch)
            || ('\u{4E00}'..='\u{9FFF}').contains(&ch)
            || ('\u{F900}'..='\u{FAFF}').contains(&ch)
            || ('\u{20000}'..='\u{2A6DF}').contains(&ch)
            || ('\u{2A700}'..='\u{2B73F}').contains(&ch)
            || ('\u{2B740}'..='\u{2B81F}').contains(&ch)
            || ('\u{2B820}'..='\u{2CEAF}').contains(&ch)
            || ('\u{2CEB0}'..='\u{2EBEF}').contains(&ch)
            || ('\u{30000}'..='\u{3134F}').contains(&ch)
            || ('\u{31350}'..='\u{323AF}').contains(&ch)
    })
}

fn build_client(user_agent: &str) -> Result<Client, YdtError> {
    Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(YdtError::CreateHttpClient)
}

fn send_with_ua(word: &str, user_agent: &str) -> Result<Response, YdtError> {
    let client = build_client(user_agent)?;
    let url = Url::parse_with_params(YOUDAO_RESULT_URL, &[("word", word), ("lang", "en")])
        .map_err(YdtError::BuildRequestUrl)?;
    client.get(url).send().map_err(YdtError::FetchTranslation)
}

fn ensure_success_response(response: Response) -> Result<Response, YdtError> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(YdtError::HttpStatus(status))
    }
}

fn fetch_with_fallback(word: &str) -> Result<Response, YdtError> {
    match send_with_ua(word, PROJECT_USER_AGENT) {
        Ok(resp) => {
            let status = resp.status();
            if status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS {
                let fallback_resp = send_with_ua(word, BROWSER_USER_AGENT)?;
                ensure_success_response(fallback_resp)
            } else {
                ensure_success_response(resp)
            }
        }
        Err(_) => {
            let fallback_resp = send_with_ua(word, BROWSER_USER_AGENT)?;
            ensure_success_response(fallback_resp)
        }
    }
}

fn cached_selector(
    cache: &'static OnceLock<Result<Selector, YdtError>>,
    css: &'static str,
) -> Result<&'static Selector, YdtError> {
    match cache.get_or_init(|| Selector::parse(css).map_err(|_| YdtError::ParseCssSelector(css))) {
        Ok(selector) => Ok(selector),
        Err(_) => Err(YdtError::ParseCssSelector(css)),
    }
}

/// Parse translation text from a Youdao result HTML fragment.
///
/// This function does not perform network I/O.
///
/// # Examples
///
/// ```
/// let html = r#"
/// <div class="trans-container">
///   <div class="per-phone">
///     <span>英</span><span class="phonetic">/həˈləʊ/</span>
///   </div>
/// </div>
/// <div class="trans-container">
///   <li class="word-exp">
///     <span class="pos">int.</span>
///     <span class="trans">你好</span>
///   </li>
/// </div>
/// "#;
/// let out = ydt::parse_translation_from_html("hello", html).unwrap();
/// assert_eq!(out, "英 /həˈləʊ/\nint.: 你好");
/// ```
pub fn parse_translation_from_html(word: &str, html: &str) -> Result<String, YdtError> {
    let document = Html::parse_document(html);
    let mut translations = Vec::new();
    let mut phonetics = Vec::new();

    if contains_cjk_ideograph(word) {
        let word_exp_selector =
            cached_selector(&WORD_EXP_CE_SELECTOR, "li.word-exp-ce.mcols-layout")?;
        let point_selector = cached_selector(&POINT_SELECTOR, "a.point")?;

        for exp in document.select(word_exp_selector) {
            if let Some(word_text) = exp.select(point_selector).next() {
                translations.push(word_text.text().collect::<String>());
            }
        }
    } else {
        let trans_container_selector =
            cached_selector(&TRANS_CONTAINER_SELECTOR, "div.trans-container")?;
        let phone_selector = cached_selector(&PHONE_SELECTOR, "div.per-phone")?;
        let span_selector = cached_selector(&SPAN_SELECTOR, "span")?;
        let phonetic_selector = cached_selector(&PHONETIC_SELECTOR, "span.phonetic")?;
        let word_exp_selector = cached_selector(&WORD_EXP_SELECTOR, "li.word-exp")?;
        let pos_selector = cached_selector(&POS_SELECTOR, "span.pos")?;
        let trans_selector = cached_selector(&TRANS_SELECTOR, "span.trans")?;

        if let Some(container) = document.select(trans_container_selector).nth(0) {
            for phone_div in container.select(phone_selector) {
                if let Some(label) = phone_div.select(span_selector).next() {
                    let label_text = label.text().collect::<String>().trim().to_string();
                    if let Some(phonetic) = phone_div.select(phonetic_selector).next() {
                        let phonetic_text = phonetic.text().collect::<String>().trim().to_string();
                        phonetics.push(format!("{} {}", label_text, phonetic_text));
                    }
                }
            }
        }

        if let Some(container) = document.select(trans_container_selector).nth(1) {
            for exp in container.select(word_exp_selector) {
                if let (Some(pos), Some(trans)) = (
                    exp.select(pos_selector).next(),
                    exp.select(trans_selector).next(),
                ) {
                    let pos_text = pos.text().collect::<String>().trim().to_string();
                    let trans_text = trans.text().collect::<String>().trim().to_string();
                    translations.push(format!("{}: {}", pos_text, trans_text));
                }
            }
        }
    }

    if phonetics.is_empty() && translations.is_empty() {
        Ok("No results.".to_string())
    } else {
        let phonetics_str = phonetics.join(" ");
        let translations_str = translations.join("\n");
        if phonetics_str.is_empty() {
            Ok(translations_str)
        } else if translations_str.is_empty() {
            Ok(phonetics_str)
        } else {
            Ok(format!("{}\n{}", phonetics_str, translations_str))
        }
    }
}

/// Fetch translation for a word from Youdao and return normalized display text.
///
/// # Errors
///
/// Returns [`YdtError`] when request building, HTTP request, HTTP status validation,
/// response reading, or selector parsing fails.
pub fn get_translation(word: &str) -> Result<String, YdtError> {
    let response = fetch_with_fallback(word)?;
    let html = response.text().map_err(YdtError::ReadResponse)?;
    parse_translation_from_html(word, &html)
}
