use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use reqwest::Url;
use scraper::{Html, Selector};
use std::env;
use std::sync::OnceLock;
use std::time::Duration;

const PROJECT_USER_AGENT: &str = concat!(
    "ydt/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/yushengyangchem/ydt)"
);
const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";
const YOUDAO_RESULT_URL: &str = "https://www.youdao.com/result";

static WORD_EXP_CE_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static POINT_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static TRANS_CONTAINER_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static PHONE_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static SPAN_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static PHONETIC_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static WORD_EXP_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static POS_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();
static TRANS_SELECTOR: OnceLock<Result<Selector, String>> = OnceLock::new();

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

fn build_client(user_agent: &str) -> Result<Client, String> {
    Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "Failed to create HTTP client.".to_string())
}

fn send_with_ua(word: &str, user_agent: &str) -> Result<Response, String> {
    let client = build_client(user_agent)?;
    let url = Url::parse_with_params(YOUDAO_RESULT_URL, &[("word", word), ("lang", "en")])
        .map_err(|_| "Failed to build request URL.".to_string())?;
    client
        .get(url)
        .send()
        .map_err(|_| "Failed to fetch translation.".to_string())
}

fn ensure_success_response(response: Response) -> Result<Response, String> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        Err(format!("Request failed with status: {status}"))
    }
}

fn fetch_with_fallback(word: &str) -> Result<Response, String> {
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
    cache: &'static OnceLock<Result<Selector, String>>,
    css: &'static str,
) -> Result<&'static Selector, String> {
    match cache
        .get_or_init(|| Selector::parse(css).map_err(|_| format!("Failed to parse CSS selector: {css}")))
    {
        Ok(selector) => Ok(selector),
        Err(err) => Err(err.clone()),
    }
}

fn get_translation(word: &str) -> String {
    let response = match fetch_with_fallback(word) {
        Ok(resp) => resp,
        Err(err) => return err,
    };

    let html = match response.text() {
        Ok(text) => text,
        Err(_) => return "Failed to read response.".to_string(),
    };

    let document = Html::parse_document(&html);
    let mut translations = Vec::new();
    let mut phonetics = Vec::new();

    if contains_cjk_ideograph(word) {
        let word_exp_selector = match cached_selector(&WORD_EXP_CE_SELECTOR, "li.word-exp-ce.mcols-layout") {
            Ok(selector) => selector,
            Err(err) => return err,
        };
        let point_selector = match cached_selector(&POINT_SELECTOR, "a.point") {
            Ok(selector) => selector,
            Err(err) => return err,
        };

        for exp in document.select(&word_exp_selector) {
            if let Some(word_text) = exp.select(&point_selector).next() {
                translations.push(word_text.text().collect::<String>());
            }
        }
    } else {
        let trans_container_selector = match cached_selector(&TRANS_CONTAINER_SELECTOR, "div.trans-container")
        {
            Ok(selector) => selector,
            Err(err) => return err,
        };
        let phone_selector = match cached_selector(&PHONE_SELECTOR, "div.per-phone") {
            Ok(selector) => selector,
            Err(err) => return err,
        };
        let span_selector = match cached_selector(&SPAN_SELECTOR, "span") {
            Ok(selector) => selector,
            Err(err) => return err,
        };
        let phonetic_selector = match cached_selector(&PHONETIC_SELECTOR, "span.phonetic") {
            Ok(selector) => selector,
            Err(err) => return err,
        };
        let word_exp_selector = match cached_selector(&WORD_EXP_SELECTOR, "li.word-exp") {
            Ok(selector) => selector,
            Err(err) => return err,
        };
        let pos_selector = match cached_selector(&POS_SELECTOR, "span.pos") {
            Ok(selector) => selector,
            Err(err) => return err,
        };
        let trans_selector = match cached_selector(&TRANS_SELECTOR, "span.trans") {
            Ok(selector) => selector,
            Err(err) => return err,
        };

        if let Some(container) = document.select(&trans_container_selector).nth(0) {
            for phone_div in container.select(&phone_selector) {
                if let Some(label) = phone_div.select(&span_selector).next() {
                    let label_text = label.text().collect::<String>().trim().to_string();
                    if let Some(phonetic) = phone_div.select(&phonetic_selector).next() {
                        let phonetic_text = phonetic.text().collect::<String>().trim().to_string();
                        phonetics.push(format!("{} {}", label_text, phonetic_text));
                    }
                }
            }
        }

        if let Some(container) = document.select(&trans_container_selector).nth(1) {
            for exp in container.select(&word_exp_selector) {
                if let (Some(pos), Some(trans)) = (
                    exp.select(&pos_selector).next(),
                    exp.select(&trans_selector).next(),
                ) {
                    let pos_text = pos.text().collect::<String>().trim().to_string();
                    let trans_text = trans.text().collect::<String>().trim().to_string();
                    translations.push(format!("{}: {}", pos_text, trans_text));
                }
            }
        }
    }

    let output = if phonetics.is_empty() && translations.is_empty() {
        "No results.".to_string()
    } else {
        let phonetics_str = phonetics.join(" ");
        let translations_str = translations.join("\n");
        if phonetics_str.is_empty() {
            translations_str
        } else if translations_str.is_empty() {
            phonetics_str
        } else {
            format!("{}\n{}", phonetics_str, translations_str)
        }
    };
    output
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Please provide a word to translate");
        return;
    }
    println!("{}", get_translation(&args[1]));
}
