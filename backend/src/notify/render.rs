//! Turns a [`Notice`] into the JSON a Discord webhook expects. Pure: no clock, no network.
//!
//! Webhooks cannot send buttons, so the links (the racer's page on the site, their channels) live
//! inside the embed. Nobody is mentioned, except the referees' role on a critical staff alert.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use super::text::{LiveInfo, escape, hms, platform_label, pretty_area, pretty_boss, safe_url};
use super::{Channel, Detail, Notice};

pub struct Ctx<'a> {
    /// The site's address without a trailing slash (empty = no links to the site).
    pub public_url: &'a str,
    /// Rehearsal: staff messages say so.
    pub rehearsal: bool,
    /// Role to mention on critical staff alerts (never on tests).
    pub role_id: Option<&'a str>,
    /// The panel's test message.
    pub test: bool,
    pub now: DateTime<Utc>,
}

const BLUE: u32 = 0x3DDCFF;
const GOLD: u32 = 0xFFD54A;
const GREEN: u32 = 0x3DDC84;
const ORANGE: u32 = 0xFF9F43;
const PURPLE: u32 = 0xB36BFF;
const RED: u32 = 0xFF4D6D;
const GREY: u32 = 0x8B93A7;

fn name_of(n: &Notice) -> String {
    n.racer
        .as_ref()
        .map(|r| escape(&r.name))
        .unwrap_or_else(|| "Un corredor".into())
}

fn progress_line(r: &LiveInfo) -> String {
    let mut line = format!("🎮 Progreso: **{}%**", r.progress_pct.round() as i64);
    if let Some(area) = r.area.as_deref().filter(|a| !a.is_empty()) {
        line.push_str(&format!(" · {}", escape(&pretty_area(area))));
    }
    line
}

fn racer_links(r: &LiveInfo, ctx: &Ctx) -> Vec<String> {
    let mut links = Vec::new();
    if !ctx.public_url.is_empty() {
        links.push(format!(
            "[Seguir en Zeldatón]({})",
            safe_url(&format!("{}/racer/{}", ctx.public_url, r.id))
        ));
    }
    for (platform, url) in &r.channels {
        links.push(format!(
            "[{}]({})",
            platform_label(*platform),
            safe_url(url)
        ));
    }
    links
}

fn thumbnail(r: &LiveInfo, ctx: &Ctx) -> Option<String> {
    let a = r.avatar_url.as_deref()?;
    if a.starts_with("https://") {
        Some(a.to_string())
    } else if a.starts_with('/') && !ctx.public_url.is_empty() {
        Some(format!("{}{a}", ctx.public_url))
    } else {
        None
    }
}

/// The embed every kind shares: title, description, colour, optional racer links and thumbnail.
fn embed(
    n: &Notice,
    ctx: &Ctx,
    title: String,
    description: String,
    color: u32,
    with_links: bool,
) -> Value {
    let staff = n.kind().channel() == Channel::Staff;
    let prefix = if staff && ctx.rehearsal {
        "[ENSAYO] "
    } else {
        ""
    };
    let footer = match (staff, ctx.test) {
        (true, true) => "Zeldatón · árbitros · mensaje de prueba",
        (true, false) => "Zeldatón · árbitros",
        (false, true) => "Zeldatón · mensaje de prueba",
        (false, false) => "Zeldatón · Ocarina of Time",
    };
    let mut e = json!({
        "title": format!("{prefix}{title}"),
        "description": description,
        "color": color,
        "footer": { "text": footer },
        "timestamp": ctx.now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    });
    if let Some(r) = &n.racer {
        if !ctx.public_url.is_empty() {
            e["url"] = json!(format!("{}/racer/{}", ctx.public_url, r.id));
        }
        if with_links {
            let mut links = racer_links(r, ctx);
            if staff && !ctx.public_url.is_empty() {
                links.push(format!(
                    "[Panel de corredores]({})",
                    safe_url(&format!("{}/admin/racers", ctx.public_url))
                ));
            }
            if !links.is_empty() {
                e["fields"] =
                    json!([{ "name": "Enlaces", "value": links.join(" · "), "inline": false }]);
            }
        }
        if let Some(t) = thumbnail(r, ctx) {
            e["thumbnail"] = json!({ "url": t });
        }
    }
    e
}

pub fn render(n: &Notice, ctx: &Ctx) -> Value {
    let name = name_of(n);
    let progress = n.racer.as_ref().map(progress_line).unwrap_or_default();
    let e = match &n.detail {
        Detail::Live => {
            let r = n.racer.as_ref();
            let mut lines = vec![progress.clone()];
            if let Some(v) = r.and_then(|r| r.viewers).filter(|v| *v > 0) {
                lines.push(format!("👁 {v} espectadores"));
            }
            let mut e = embed(
                n,
                ctx,
                format!("🔴 {name} está en vivo"),
                lines.join("\n"),
                BLUE,
                false,
            );
            if let Some(r) = r {
                let links = racer_links(r, ctx);
                if !links.is_empty() {
                    e["fields"] = json!([{ "name": "Míralo aquí", "value": links.join(" · "), "inline": false }]);
                }
            }
            e
        }
        Detail::Winner {
            place,
            final_seconds,
        } => {
            let time = final_seconds
                .map(|s| format!("Tiempo de juego: **{}**.\n", hms(s)))
                .unwrap_or_default();
            if *place <= 1 {
                embed(
                    n,
                    ctx,
                    format!("🏆 ¡{name} gana Zeldatón!"),
                    format!("{time}Es el primero en completar Ocarina of Time. ¡Felicidades!"),
                    GOLD,
                    true,
                )
            } else {
                embed(
                    n,
                    ctx,
                    format!("🏁 {name} completó el juego"),
                    format!("{time}Llega en el lugar **{place}**."),
                    GREEN,
                    true,
                )
            }
        }
        Detail::Exhausted => embed(
            n,
            ctx,
            format!("⏱️ {name} se quedó sin tiempo"),
            format!("{progress}\nVuelve a jugar cuando se reinicie su día."),
            ORANGE,
            true,
        ),
        Detail::Boss { boss, count } => {
            let mut d = progress.clone();
            if let Some(c) = count {
                d.push_str(&format!("\n👹 Jefes derrotados: **{c}**"));
            }
            embed(
                n,
                ctx,
                format!("👹 {name} derrotó a {}", escape(&pretty_boss(boss))),
                d,
                PURPLE,
                true,
            )
        }
        Detail::Leader { previous } => {
            let mut d = progress.clone();
            if let Some(p) = previous {
                d.push_str(&format!("\nAntes iba primero {}.", escape(p)));
            }
            embed(
                n,
                ctx,
                format!("🥇 {name} toma la delantera"),
                d,
                GOLD,
                true,
            )
        }
        Detail::Disconnected { minutes, back } => {
            if *back {
                embed(
                    n,
                    ctx,
                    format!("🟢 {name} volvió a conectarse"),
                    "HiveShock está conectado otra vez.".into(),
                    GREEN,
                    true,
                )
            } else {
                embed(
                    n,
                    ctx,
                    format!("🔌 {name} lleva {minutes} min desconectado"),
                    format!(
                        "{progress}\nRevisa si perdió la conexión con HiveShock o cerró el juego."
                    ),
                    RED,
                    true,
                )
            }
        }
        Detail::NoShow {
            minutes_late,
            start,
            ..
        } => embed(
            n,
            ctx,
            format!("🚫 {name} no aparece: lleva {minutes_late} min de retraso"),
            format!(
                "Su live estaba programado a las <t:{}:t> (<t:{}:R>) y no hay señal de HiveShock.",
                start.timestamp(),
                start.timestamp()
            ),
            RED,
            true,
        ),
        Detail::Uncovered {
            starts_in_minutes,
            start,
            ..
        } => embed(
            n,
            ctx,
            if *starts_in_minutes > 0 {
                format!("🧑‍⚖️ Falta árbitro para el live de {name}")
            } else {
                format!("🧑‍⚖️ El live de {name} ya empezó sin árbitro")
            },
            format!(
                "Empieza a las <t:{}:t> (<t:{}:R>). Toma la franja desde el monitor.",
                start.timestamp(),
                start.timestamp()
            ),
            ORANGE,
            true,
        ),
        Detail::LowTime { minutes_left } => embed(
            n,
            ctx,
            format!("⚠️ A {name} le quedan {minutes_left} min"),
            format!("{progress}\nSu tiempo del día está por agotarse."),
            ORANGE,
            true,
        ),
        Detail::DonationCap {
            adding,
            limit_seconds,
            viewer,
        } => {
            let (verb, what) = if *adding {
                ("sumar", "suman")
            } else {
                ("restar", "restan")
            };
            let who = viewer
                .as_deref()
                .map(|v| format!("\nÚltima donación: {}.", escape(v)))
                .unwrap_or_default();
            embed(
                n,
                ctx,
                format!("🚧 {name} alcanzó el tope diario de donaciones ({verb})"),
                format!(
                    "Tope: **{}**. Hoy las donaciones ya no {what} tiempo a este corredor.{who}",
                    hms(*limit_seconds)
                ),
                ORANGE,
                true,
            )
        }
        Detail::Jump { from, to, seconds } => embed(
            n,
            ctx,
            format!("🚩 Progreso sospechoso: {name}"),
            format!(
                "Saltó de **{}%** a **{}%** en {seconds} s. Conviene revisar su stream.",
                from.round() as i64,
                to.round() as i64
            ),
            RED,
            true,
        ),
        Detail::EventState {
            state,
            actor,
            reason,
        } => embed(
            n,
            ctx,
            format!("🔔 Evento {state}"),
            by(actor, reason.as_deref()),
            GREY,
            false,
        ),
        Detail::Organizer {
            actor,
            action,
            reason,
            detail,
        } => {
            let mut d = by(actor, reason.as_deref());
            if let Some(x) = detail {
                d.push_str(&format!("\n{}", escape(x)));
            }
            embed(
                n,
                ctx,
                format!("🛠️ {} {action} {name}", escape(actor)),
                d,
                GREY,
                true,
            )
        }
    };

    let mention = ctx
        .role_id
        .filter(|_| n.kind().critical() && !ctx.test && n.kind().channel() == Channel::Staff);
    let mut body = json!({
        "username": "Zeldatón",
        // Nobody gets pinged because of a racer's name; only the referees' role, on critical alerts.
        "allowed_mentions": { "parse": [] },
        "embeds": [e],
    });
    if let Some(role) = mention {
        body["content"] = json!(format!("<@&{role}>"));
        body["allowed_mentions"] = json!({ "parse": [], "roles": [role] });
    }
    body
}

fn by(actor: &str, reason: Option<&str>) -> String {
    match reason {
        Some(r) => format!("Por **{}** · {}", escape(actor), escape(r)),
        None => format!("Por **{}**", escape(actor)),
    }
}

/// The panel's test message for a channel: a sample of what that channel receives.
pub fn test_notice(channel: Channel, info: LiveInfo, now: DateTime<Utc>) -> Notice {
    let detail = match channel {
        Channel::Public => Detail::Live,
        Channel::Staff => Detail::Disconnected {
            minutes: 3,
            back: false,
        },
    };
    debug_assert_eq!(detail.kind().channel(), channel);
    Notice::new(Some(info), detail, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Platform;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.timestamp_opt(1_790_000_000, 0).unwrap()
    }
    fn info() -> LiveInfo {
        LiveInfo {
            id: "ralbat".into(),
            name: "Ral*bat_ @everyone [x](y)".into(),
            avatar_url: Some("/api/media/racers/ralbat-1.png".into()),
            progress_pct: 41.6,
            area: Some("water-temple".into()),
            viewers: Some(120),
            channels: vec![
                (Platform::Twitch, "https://twitch.tv/ralbat".into()),
                (Platform::Tiktok, "https://tiktok.com/@ra lbat)".into()),
            ],
        }
    }
    fn ctx() -> Ctx<'static> {
        Ctx {
            public_url: "https://zeldaton.example",
            rehearsal: false,
            role_id: None,
            test: false,
            now: now(),
        }
    }
    fn notice(d: Detail) -> Notice {
        Notice::new(Some(info()), d, now())
    }
    fn title(v: &Value) -> String {
        v["embeds"][0]["title"].as_str().unwrap().to_string()
    }

    #[test]
    fn live_links_the_site_and_each_channel() {
        let m = render(&notice(Detail::Live), &ctx());
        let e = &m["embeds"][0];
        assert_eq!(e["url"], "https://zeldaton.example/racer/ralbat");
        let links = e["fields"][0]["value"].as_str().unwrap();
        assert!(links.starts_with("[Seguir en Zeldatón](https://zeldaton.example/racer/ralbat)"));
        assert!(links.contains("[Twitch](https://twitch.tv/ralbat)"));
        assert!(links.contains("[TikTok](https://tiktok.com/@ra%20lbat%29)"));
        let d = e["description"].as_str().unwrap();
        assert!(
            d.contains("**42%**") && d.contains("Water Temple") && d.contains("120 espectadores")
        );
        assert_eq!(
            e["thumbnail"]["url"],
            "https://zeldaton.example/api/media/racers/ralbat-1.png"
        );
        assert_eq!(e["footer"]["text"], "Zeldatón · Ocarina of Time");
    }

    #[test]
    fn the_community_messages_say_the_right_thing() {
        let win = render(
            &notice(Detail::Winner {
                place: 1,
                final_seconds: Some(3725),
            }),
            &ctx(),
        );
        assert!(title(&win).starts_with("🏆 ¡") && title(&win).ends_with(" gana Zeldatón!"));
        assert!(
            win["embeds"][0]["description"]
                .as_str()
                .unwrap()
                .contains("01:02:05")
        );
        let second = render(
            &notice(Detail::Winner {
                place: 2,
                final_seconds: None,
            }),
            &ctx(),
        );
        assert!(
            title(&second).starts_with("🏁")
                && second["embeds"][0]["description"]
                    .as_str()
                    .unwrap()
                    .contains("lugar **2**")
        );
        let boss = render(
            &notice(Detail::Boss {
                boss: "king-dodongo".into(),
                count: Some(2),
            }),
            &ctx(),
        );
        assert!(title(&boss).ends_with("derrotó a Rey Dodongo"));
        assert!(
            boss["embeds"][0]["description"]
                .as_str()
                .unwrap()
                .contains("Jefes derrotados: **2**")
        );
        assert!(title(&render(&notice(Detail::Exhausted), &ctx())).contains("se quedó sin tiempo"));
        let lead = render(
            &notice(Detail::Leader {
                previous: Some("Cuaco".into()),
            }),
            &ctx(),
        );
        assert!(title(&lead).contains("toma la delantera"));
        assert!(
            lead["embeds"][0]["description"]
                .as_str()
                .unwrap()
                .contains("Antes iba primero Cuaco")
        );
    }

    #[test]
    fn staff_messages_carry_the_panel_link_and_the_rehearsal_tag() {
        let n = notice(Detail::Disconnected {
            minutes: 5,
            back: false,
        });
        let live = render(&n, &ctx());
        assert!(
            title(&live).contains("lleva 5 min desconectado")
                && !title(&live).starts_with("[ENSAYO]")
        );
        let links = live["embeds"][0]["fields"][0]["value"].as_str().unwrap();
        assert!(links.contains("[Panel de corredores](https://zeldaton.example/admin/racers)"));
        assert_eq!(live["embeds"][0]["footer"]["text"], "Zeldatón · árbitros");
        let rehearsal = render(
            &n,
            &Ctx {
                rehearsal: true,
                ..ctx()
            },
        );
        assert!(title(&rehearsal).starts_with("[ENSAYO] "));
        // The community channel never says "ensayo" (it is not even sent in rehearsal).
        let public = render(
            &notice(Detail::Exhausted),
            &Ctx {
                rehearsal: true,
                ..ctx()
            },
        );
        assert!(!title(&public).contains("ENSAYO"));
    }

    #[test]
    fn nobody_is_pinged_except_the_referees_role_on_a_critical_alert() {
        let critical = notice(Detail::LowTime { minutes_left: 9 });
        let plain = render(&critical, &ctx());
        assert_eq!(plain["allowed_mentions"], json!({ "parse": [] }));
        assert!(plain.get("content").is_none());
        let with_role = render(
            &critical,
            &Ctx {
                role_id: Some("123456789012345678"),
                ..ctx()
            },
        );
        assert_eq!(with_role["content"], "<@&123456789012345678>");
        assert_eq!(
            with_role["allowed_mentions"],
            json!({ "parse": [], "roles": ["123456789012345678"] })
        );
        // Not critical, a public message, or a test: no mention even with the role set.
        for n in [
            notice(Detail::DonationCap {
                adding: false,
                limit_seconds: 3600,
                viewer: None,
            }),
            notice(Detail::Exhausted),
        ] {
            let m = render(
                &n,
                &Ctx {
                    role_id: Some("123456789012345678"),
                    ..ctx()
                },
            );
            assert!(m.get("content").is_none(), "{:?}", n.kind());
        }
        let test = render(
            &critical,
            &Ctx {
                role_id: Some("123456789012345678"),
                test: true,
                ..ctx()
            },
        );
        assert!(test.get("content").is_none());
        assert_eq!(
            test["embeds"][0]["footer"]["text"],
            "Zeldatón · árbitros · mensaje de prueba"
        );
    }

    #[test]
    fn hostile_names_cannot_ping_or_format() {
        let m = render(&notice(Detail::Exhausted), &ctx());
        assert!(
            title(&m).contains("Ral\\*bat\\_ @\u{200b}everyone \\[x\\]\\(y\\)"),
            "{}",
            title(&m)
        );
        let org = render(
            &notice(Detail::Organizer {
                actor: "ana".into(),
                action: "ajustó el tiempo de".into(),
                reason: Some("@everyone *lag*".into()),
                detail: Some("+00:05:00".into()),
            }),
            &ctx(),
        );
        let d = org["embeds"][0]["description"].as_str().unwrap();
        assert!(d.contains("@\u{200b}everyone \\*lag\\*") && d.contains("+00:05:00"));
    }

    #[test]
    fn it_degrades_without_site_url_channels_avatar_or_racer() {
        let mut bare = info();
        bare.channels.clear();
        bare.avatar_url = None;
        bare.area = None;
        let n = Notice::new(Some(bare), Detail::Exhausted, now());
        let m = render(
            &n,
            &Ctx {
                public_url: "",
                ..ctx()
            },
        );
        let e = &m["embeds"][0];
        assert!(
            e.get("url").is_none() && e.get("fields").is_none() && e.get("thumbnail").is_none()
        );
        let ev = render(
            &Notice::new(
                None,
                Detail::EventState {
                    state: "iniciado".into(),
                    actor: "ana".into(),
                    reason: None,
                },
                now(),
            ),
            &ctx(),
        );
        assert_eq!(title(&ev), "🔔 Evento iniciado");
        assert_eq!(ev["embeds"][0]["description"], "Por **ana**");
    }

    #[test]
    fn each_channel_has_a_test_message_of_its_own_kind() {
        for ch in Channel::ALL {
            assert_eq!(test_notice(ch, info(), now()).kind().channel(), ch);
        }
    }
}
