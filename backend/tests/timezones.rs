//! The organizer panel offers a fixed list of time zones of the Americas (src/config/timezones.ts).
//! Every one must be accepted by the backend, or saving a racer would fail with "timezone must be an
//! IANA name".

use chrono_tz::Tz;

#[test]
fn every_time_zone_the_panel_offers_is_accepted() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/config/timezones.ts");
    let source = std::fs::read_to_string(path).expect("src/config/timezones.ts");
    let ids: Vec<&str> = source
        .split("id: '")
        .skip(1)
        .filter_map(|rest| rest.split('\'').next())
        .collect();
    assert!(ids.len() >= 40, "only {} zones found", ids.len());
    for id in &ids {
        assert!(id.parse::<Tz>().is_ok(), "chrono-tz does not know {id}");
    }
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "a zone is listed twice");
    assert!(
        ids.contains(&"America/Tijuana"),
        "Mexicali (America/Tijuana) must be offered"
    );
}
