use std::time::Duration;

use crate::yandex::{urlencode, LyricsDto, LyricsLineDto};

const BETTER_URL: &str = "https://lyrics-api.boidu.dev/getLyrics";

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

fn strip_tags(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut depth = 0;
    for ch in raw.chars() {
        match ch {
            '<' => depth += 1,
            '>' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out
}

fn attr<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    let pattern = format!("{name}=\"");
    let start = head.find(&pattern)? + pattern.len();
    let rest = &head[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// TTML из Apple Music → строки с временем начала (словесные тайминги нам не нужны).
fn decode_entities(raw: &str) -> String {
    raw.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

/// TTML из Apple Music → строки с временем начала. Пробелы между span'ами и
/// есть границы слов, внутри слова слоги идут подряд.
fn ttml_lines(ttml: &str) -> Vec<LyricsLineDto> {
    let mut lines: Vec<LyricsLineDto> = Vec::new();
    for chunk in ttml.split("</p>") {
        let Some(start) = chunk.rfind("<p") else {
            continue;
        };
        let Some(gt) = chunk[start..].find('>') else {
            continue;
        };
        let head = &chunk[start..start + gt];
        let Some(begin) = attr(head, "begin").and_then(|v| v.parse::<f64>().ok()) else {
            continue;
        };
        let text = decode_entities(&strip_tags(&chunk[start + gt + 1..]))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            continue;
        }
        lines.push(LyricsLineDto {
            time_ms: (begin * 1000.0).round() as i64,
            text,
        });
    }
    lines.sort_by_key(|line| line.time_ms);
    lines
}

fn dto(track_id: &str, lines: Vec<LyricsLineDto>) -> Option<LyricsDto> {
    if lines.len() < 2 {
        return None;
    }
    let synced = lines.iter().any(|line| line.time_ms > 0);
    Some(LyricsDto {
        track_id: track_id.to_string(),
        synced,
        lines,
        writers: Vec::new(),
    })
}

#[tauri::command]
pub async fn better_lyrics(
    id: String,
    title: String,
    artist: String,
    duration: Option<u32>,
) -> Result<Option<LyricsDto>, String> {
    let Some(duration) = duration.filter(|value| *value > 0) else {
        return Ok(None);
    };
    let response = client()
        .get(BETTER_URL)
        .query(&[
            ("s", title.as_str()),
            ("a", artist.as_str()),
            ("d", &duration.to_string()),
        ])
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    let ttml = body.get("ttml").and_then(|v| v.as_str()).unwrap_or("");
    if ttml.is_empty() {
        return Ok(None);
    }
    Ok(dto(&id, ttml_lines(ttml)))
}

const MXM_SITE: &str = "https://www.musixmatch.com";
const MXM_APP: &str = "community-app-v1.0";
const MXM_KEY: &str = "13d526195ebbdcddc40c861c4df22ba5";

fn utc_day() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64 + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}{month:02}{day:02}")
}

fn mxm_sign(url: &str) -> String {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(MXM_KEY.as_bytes()).expect("hmac key");
    mac.update(format!("{url}{}", utc_day()).as_bytes());
    BASE64.encode(mac.finalize().into_bytes())
}

async fn mxm_api(
    http: &reqwest::Client,
    method: &str,
    params: &[(&str, String)],
) -> Option<serde_json::Value> {
    let mut url = url::Url::parse(&format!("{MXM_SITE}/ws/1.1/{method}")).ok()?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("app_id", MXM_APP);
        query.append_pair("format", "json");
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }
    let signed = format!(
        "{}&signature={}&signature_protocol=sha256",
        url.as_str(),
        urlencode(&mxm_sign(url.as_str()))
    );
    let response = http
        .get(signed)
        .header("accept", "application/json")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: serde_json::Value = response.json().await.ok()?;
    let message = body.get("message")?;
    if message.get("header").and_then(|h| h.get("status_code")).and_then(|v| v.as_i64()) != Some(200) {
        return None;
    }
    message.get("body").cloned()
}

fn clean_plain(body: &str) -> Vec<LyricsLineDto> {
    body.lines()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.trim_start().starts_with("****"))
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.trim().chars().all(|ch| ch.is_ascii_digit() || ch == '.'))
        .map(|text| LyricsLineDto { time_ms: 0, text })
        .collect()
}

#[tauri::command]
pub async fn musixmatch(id: String, title: String, artist: String) -> Result<Option<LyricsDto>, String> {
    let http = client();
    let search = mxm_api(
        &http,
        "track.search",
        &[
            ("q_track", title),
            ("q_artist", artist),
            ("page_size", "5".to_string()),
        ],
    )
    .await;
    let Some(search) = search else {
        return Ok(None);
    };
    let track_id = search
        .get("track_list")
        .and_then(|list| list.as_array())
        .and_then(|list| list.first())
        .and_then(|item| item.get("track"))
        .and_then(|track| track.get("track_id"))
        .and_then(|v| v.as_i64());
    let Some(track_id) = track_id else {
        return Ok(None);
    };
    let lyrics = mxm_api(
        &http,
        "track.lyrics.get",
        &[("track_id", track_id.to_string())],
    )
    .await;
    let Some(lyrics) = lyrics else {
        return Ok(None);
    };
    let body = lyrics
        .get("lyrics")
        .and_then(|l| l.get("lyrics_body"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let lines = clean_plain(body);
    if lines.len() < 2 {
        return Ok(None);
    }
    Ok(Some(LyricsDto {
        track_id: id,
        synced: false,
        lines,
        writers: Vec::new(),
    }))
}
