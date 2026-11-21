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
        let inner = &chunk[start + gt + 1..];
        let begin = attr(head, "begin")
            .and_then(parse_begin)
            .or_else(|| {
                let span = &inner[inner.find("<span")?..];
                let span_head = &span[..span.find('>')?];
                attr(span_head, "begin").and_then(parse_begin)
            });
        let Some(begin) = begin else {
            continue;
        };
        let text = decode_entities(&strip_tags(inner))
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if text.is_empty() {
            continue;
        }
        lines.push(LyricsLineDto {
            time_ms: (begin * 1000.0).round() as i64,
            text,
            words: Vec::new(),
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

fn parse_begin(value: &str) -> Option<f64> {
    if let Ok(seconds) = value.parse::<f64>() {
        return Some(seconds);
    }
    let parts: Vec<&str> = value.split(':').collect();
    let mut seconds = 0.0f64;
    for part in parts {
        seconds = seconds * 60.0 + part.parse::<f64>().ok()?;
    }
    Some(seconds)
}

async fn better_request(
    http: &reqwest::Client,
    endpoint: &str,
    title: &str,
    artist: &str,
    album: Option<&str>,
    duration: Option<u32>,
) -> Option<Vec<LyricsLineDto>> {
    let mut url = url::Url::parse(&format!("https://lyrics-api.boidu.dev/{endpoint}")).ok()?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("s", title);
        query.append_pair("a", artist);
        if let Some(album) = album.filter(|value| !value.is_empty()) {
            query.append_pair("al", album);
        }
        if let Some(duration) = duration {
            query.append_pair("d", &duration.to_string());
        }
    }
    let response = http
        .get(url)
        .header("accept", "application/json")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: serde_json::Value = response.json().await.ok()?;
    let ttml = body.get("ttml").and_then(|value| value.as_str())?;
    let lines = ttml_lines(ttml);
    if lines.len() < 2 {
        None
    } else {
        Some(lines)
    }
}

#[tauri::command]
pub async fn better_lyrics(
    id: String,
    title: String,
    artist: String,
    album: Option<String>,
    duration: Option<u32>,
) -> Result<Option<LyricsDto>, String> {
    let http = client();
    let duration = duration.filter(|value| *value > 0);
    let mut best: Vec<LyricsLineDto> = Vec::new();
    let attempts = [
        ("getLyrics", duration),
        ("getLyrics", None),
        ("ttml/getLyrics", duration),
        ("ttml/getLyrics", None),
    ];
    for (endpoint, seconds) in attempts {
        if let Some(lines) = better_request(
            &http,
            endpoint,
            &title,
            &artist,
            album.as_deref(),
            seconds,
        )
        .await
        {
            if lines.len() > best.len() {
                best = lines;
            }
            if best.len() >= 8 {
                break;
            }
        }
    }
    Ok(dto(&id, best))
}

const MXM_SITE: &str = "https://www.musixmatch.com";
const MXM_APP: &str = "mobile-app-v1.0";
const MXM_KEY: &str = "ed75e0237e1c746840647c555738a1e9";

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

const MXM_PAGE_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36";

static MXM_KEY_STATE: std::sync::OnceLock<std::sync::Mutex<Option<String>>> =
    std::sync::OnceLock::new();

fn mxm_key_state() -> &'static std::sync::Mutex<Option<String>> {
    MXM_KEY_STATE.get_or_init(|| std::sync::Mutex::new(None))
}

fn mxm_current_key() -> String {
    mxm_key_state()
        .lock()
        .ok()
        .and_then(|value| value.clone())
        .unwrap_or_else(|| MXM_KEY.to_string())
}

fn decode_key_blob(blob: &str) -> Option<String> {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine as _;

    let reversed: String = blob.chars().rev().collect();
    let mut padded = reversed.clone();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    let raw = BASE64.decode(padded.as_bytes()).ok()?;
    let text = String::from_utf8(raw).ok()?;
    let key = text.trim();
    if key.len() == 32 && key.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(key.to_string())
    } else {
        None
    }
}

fn key_from_bundle(js: &str) -> Option<String> {
    let marker = "Buffer.from(\"";
    let mut rest = js;
    while let Some(pos) = rest.find(marker) {
        let after = &rest[pos + marker.len()..];
        let Some(end) = after.find('"') else {
            break;
        };
        let blob = &after[..end];
        let tail = &after[end..after.len().min(end + 120)];
        if tail.contains("split(") && tail.contains("reverse") && tail.contains("base64") {
            if let Some(key) = decode_key_blob(blob) {
                return Some(key);
            }
        }
        rest = &after[end..];
    }
    None
}

async fn mxm_refresh_key(http: &reqwest::Client) -> Option<String> {
    let page = http
        .get(MXM_SITE)
        .header("user-agent", MXM_PAGE_UA)
        .header(
            "accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        )
        .header("accept-language", "en-US,en;q=0.9")
        .header("upgrade-insecure-requests", "1")
        .header("sec-fetch-dest", "document")
        .header("sec-fetch-mode", "navigate")
        .header("sec-fetch-site", "none")
        .header("cookie", "mxm_bab=AB")
        .send()
        .await
        .ok()?;
    if !page.status().is_success() {
        return None;
    }
    let html = page.text().await.ok()?;
    let mut urls: Vec<String> = Vec::new();
    let mut rest = html.as_str();
    while let Some(pos) = rest.find("src=\"") {
        let after = &rest[pos + 5..];
        let Some(end) = after.find('"') else {
            break;
        };
        let url = &after[..end];
        if url.contains(".js") && !urls.iter().any(|old| old == url) {
            urls.push(url.to_string());
        }
        rest = &after[end..];
        if urls.len() >= 24 {
            break;
        }
    }
    for url in urls {
        let full = if url.starts_with("http") {
            url
        } else {
            format!("{MXM_SITE}{url}")
        };
        let Ok(script) = http
            .get(full)
            .header("user-agent", MXM_PAGE_UA)
            .send()
            .await
        else {
            continue;
        };
        if !script.status().is_success() {
            continue;
        }
        let Ok(text) = script.text().await else {
            continue;
        };
        if let Some(key) = key_from_bundle(&text) {
            if let Ok(mut state) = mxm_key_state().lock() {
                *state = Some(key.clone());
            }
            return Some(key);
        }
    }
    None
}

fn mxm_headers(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header("user-agent", MXM_PAGE_UA)
        .header("accept", "application/json")
        .header("accept-language", "en-US,en;q=0.9")
        .header("referer", "https://www.musixmatch.com/")
        .header("origin", "https://www.musixmatch.com")
        .header("sec-ch-ua", "\"Chromium\";v=\"152\", \"Not?A_Brand\";v=\"24\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Windows\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-origin")
        .header("cookie", "mxm_bab=AB")
}

async fn mxm_api(
    http: &reqwest::Client,
    method: &str,
    params: &[(&str, String)],
) -> Option<serde_json::Value> {
    let mut refreshed = false;
    for attempt in 0..3u32 {
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
        let response = mxm_headers(http.get(signed)).send().await.ok()?;
        let status = response.status().as_u16();
        // WAF: 202/405 — временная блокировка выхода, ждём и пробуем снова
        if status == 202 || status == 405 {
            if attempt < 2 {
                tokio::time::sleep(std::time::Duration::from_millis(900 + 600 * attempt as u64)).await;
                continue;
            }
            return None;
        }
        if !response.status().is_success() {
            return None;
        }
        let body: serde_json::Value = response.json().await.ok()?;
        let header = body.get("message")?.get("header")?;
        let code = header.get("status_code").and_then(|v| v.as_i64());
        if code == Some(200) {
            return body.get("message")?.get("body").cloned();
        }
        let hint = header.get("hint").and_then(|v| v.as_str()).unwrap_or("");
        // ключ подписи ротируется с деплоем фронтенда — перечитываем из бандла
        if !refreshed && (code == Some(401) || hint.contains("captcha") || hint.contains("renew")) {
            refreshed = true;
            if mxm_refresh_key(http).await.is_some() {
                continue;
            }
        }
        break;
    }
    None
}

fn richsync_lines(body: &str) -> Vec<LyricsLineDto> {
    use crate::yandex::LyricsWordDto;

    let Ok(data) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(entries) = data.as_array() else {
        return Vec::new();
    };
    let mut out: Vec<LyricsLineDto> = Vec::new();
    for entry in entries {
        let ts = entry.get("ts").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let chunks = entry
            .get("l")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut words: Vec<LyricsWordDto> = Vec::new();
        let mut joined = String::new();
        for chunk in &chunks {
            let text = chunk.get("c").and_then(|v| v.as_str()).unwrap_or("");
            joined.push_str(text);
            let offset = chunk.get("o").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if text.trim().is_empty() {
                continue;
            }
            words.push(LyricsWordDto {
                time_ms: ((ts + offset) * 1000.0).round() as i64,
                text: text.trim().to_string(),
            });
        }
        let text = entry
            .get("x")
            .and_then(|v| v.as_str())
            .map(|value| value.to_string())
            .unwrap_or_else(|| joined.split_whitespace().collect::<Vec<_>>().join(" "));
        if text.trim().is_empty() {
            continue;
        }
        out.push(LyricsLineDto {
            time_ms: (ts * 1000.0).round() as i64,
            text,
            words,
        });
    }
    out
}

fn clean_plain(body: &str) -> Vec<LyricsLineDto> {
    body.lines()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.trim_start().starts_with("****"))
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !line.trim().chars().all(|ch| ch.is_ascii_digit() || ch == '.'))
        .map(|text| LyricsLineDto {
            time_ms: 0,
            text,
            words: Vec::new(),
        })
        .collect()
}

fn norm(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn pick_track(list: &[serde_json::Value], title: &str, artist: &str, duration: u32) -> Option<i64> {
    let wanted_title = norm(title);
    let wanted_artist = norm(artist);
    let mut best: Option<(i64, i32)> = None;
    for item in list {
        let Some(track) = item.get("track") else {
            continue;
        };
        let track_title = norm(track.get("track_name").and_then(|v| v.as_str()).unwrap_or(""));
        let track_artist = norm(track.get("artist_name").and_then(|v| v.as_str()).unwrap_or(""));
        if track_title.is_empty() || track_artist.is_empty() {
            continue;
        }
        let title_score = if track_title == wanted_title {
            100
        } else if track_title.contains(&wanted_title) || wanted_title.contains(&track_title) {
            60
        } else {
            0
        };
        let artist_score = if track_artist == wanted_artist {
            60
        } else if track_artist.contains(&wanted_artist) || wanted_artist.contains(&track_artist) {
            30
        } else {
            0
        };
        if title_score == 0 || artist_score == 0 {
            continue;
        }
        let length = track.get("track_length").and_then(|v| v.as_i64()).unwrap_or(0);
        let duration_score = if length > 0 && duration > 0 {
            let delta = (length - duration as i64).abs();
            if delta <= 3 {
                25
            } else if delta <= 10 {
                12
            } else {
                0
            }
        } else {
            0
        };
        let score = title_score + artist_score + duration_score;
        let id = track.get("track_id").and_then(|v| v.as_i64());
        if let Some(id) = id {
            if best.map(|(_, value)| score > value).unwrap_or(true) {
                best = Some((id, score));
            }
        }
    }
    match best {
        Some((id, score)) if score >= 90 => Some(id),
        _ => None,
    }
}

#[tauri::command]
pub async fn musixmatch(
    id: String,
    title: String,
    artist: String,
    duration: Option<u32>,
) -> Result<Option<LyricsDto>, String> {
    let http = client();
    let search = mxm_api(
        &http,
        "track.search",
        &[
            ("q_track", title.clone()),
            ("q_artist", artist.clone()),
            ("page_size", "10".to_string()),
        ],
    )
    .await;
    let Some(search) = search else {
        return Ok(None);
    };
    let list = search
        .get("track_list")
        .and_then(|value| value.as_array())
        .map(|value| value.as_slice())
        .unwrap_or(&[]);
    let Some(track_id) = pick_track(list, &title, &artist, duration.unwrap_or(0)) else {
        return Ok(None);
    };
    let track = track_id.to_string();
    if let Some(rich) = mxm_api(&http, "track.richsync.get", &[("track_id", track.clone())]).await {
        if let Some(body) = rich
            .get("richsync")
            .and_then(|value| value.get("richsync_body"))
            .and_then(|value| value.as_str())
        {
            let lines = richsync_lines(body);
            if lines.len() >= 2 {
                return Ok(Some(LyricsDto {
                    track_id: id,
                    synced: true,
                    lines,
                    writers: Vec::new(),
                }));
            }
        }
    }

    if let Some(sub) = mxm_api(
        &http,
        "track.subtitle.get",
        &[
            ("track_id", track.clone()),
            ("subtitle_format", "lrc".to_string()),
        ],
    )
    .await
    {
        if let Some(body) = sub
            .get("subtitle")
            .and_then(|value| value.get("subtitle_body"))
            .and_then(|value| value.as_str())
        {
            let lines = crate::yandex::parse_lrc(body);
            if lines.len() >= 2 {
                return Ok(Some(LyricsDto {
                    track_id: id,
                    synced: true,
                    lines,
                    writers: Vec::new(),
                }));
            }
        }
    }

    let lyrics = mxm_api(&http, "track.lyrics.get", &[("track_id", track)]).await;
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
