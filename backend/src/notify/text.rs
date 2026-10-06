//! Text helpers shared by every message: what we know about a racer, and safe formatting of the
//! strings a racer or a viewer controls (names, handles, URLs).

use crate::domain::Platform;
use crate::state::RaceState;

/// What a message needs to know about a racer, copied out of the race state.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveInfo {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub progress_pct: f64,
    pub area: Option<String>,
    pub viewers: Option<i64>,
    /// (platform, url) of each channel the racer has.
    pub channels: Vec<(Platform, String)>,
}

impl LiveInfo {
    pub fn from_state(s: &RaceState, i: usize) -> Self {
        let r = &s.racers[i].racer;
        Self {
            id: r.id.clone(),
            name: r.display_name.clone(),
            avatar_url: r.avatar_url.clone(),
            progress_pct: r.progress_percentage,
            area: r.current_area.clone(),
            viewers: r.stream.as_ref().and_then(|st| st.viewers),
            channels: r
                .channels
                .iter()
                .map(|c| (c.platform, c.url.clone()))
                .collect(),
        }
    }
}

/// Makes a racer-controlled string safe inside Discord markdown: no formatting, no mention syntax.
pub fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().take(60) {
        match c {
            '\\' | '*' | '_' | '~' | '`' | '|' | '>' | '<' | '[' | ']' | '(' | ')' | '#' => {
                out.push('\\');
                out.push(c);
            }
            '@' => {
                out.push('@');
                out.push('\u{200b}');
            }
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// A URL that cannot break out of a markdown link.
pub fn safe_url(url: &str) -> String {
    url.replace(' ', "%20")
        .replace('(', "%28")
        .replace(')', "%29")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace('[', "%5B")
        .replace(']', "%5D")
}

pub fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Twitch => "Twitch",
        Platform::Tiktok => "TikTok",
        Platform::Youtube => "YouTube",
    }
}

/// "water-temple" -> "Water Temple" (the server does not know the site's translations).
pub fn pretty_area(id: &str) -> String {
    id.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Spanish names of the bosses the site knows; anything else is shown title-cased.
pub fn pretty_boss(id: &str) -> String {
    match id {
        "gohma" => "Gohma".into(),
        "king-dodongo" => "Rey Dodongo".into(),
        "barinade" => "Barinade".into(),
        "phantom-ganon" => "Ganon Fantasma".into(),
        "volvagia" => "Volvagia".into(),
        "morpha" => "Morpha".into(),
        "bongo-bongo" => "Bongo Bongo".into(),
        "twinrova" => "Twinrova".into(),
        "ganondorf" => "Ganondorf".into(),
        "ganon" => "Ganon".into(),
        other => pretty_area(other),
    }
}

/// HH:MM:SS (hours are not wrapped at 24).
pub fn hms(total_seconds: i64) -> String {
    let s = total_seconds.max(0);
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_pretty_and_safe() {
        assert_eq!(pretty_boss("king-dodongo"), "Rey Dodongo");
        assert_eq!(pretty_boss("dark-link"), "Dark Link");
        assert_eq!(hms(3725), "01:02:05");
        assert_eq!(hms(-4), "00:00:00");
        assert_eq!(
            escape("a*b_c @all [x](y)"),
            "a\\*b\\_c @\u{200b}all \\[x\\]\\(y\\)"
        );
        assert_eq!(safe_url("https://x.y/a b)"), "https://x.y/a%20b%29");
    }
}
