//! Game catalog: the items and objectives a racer can collect, each tagged with the Link that
//! can use/reach it (child, adult or both). It lives in the database and is edited from the
//! organizer panel; this module holds the types, the validation and the factory default that
//! seeds an empty database. Ids are what HiveShock's telemetry reports.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Which Link an item or objective belongs to.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Age {
    Child,
    Adult,
    Both,
}

impl Age {
    pub fn as_str(self) -> &'static str {
        match self {
            Age::Child => "child",
            Age::Adult => "adult",
            Age::Both => "both",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "child" => Age::Child,
            "adult" => Age::Adult,
            _ => Age::Both,
        }
    }
}

/// Categories an item can be filed under (used to group the grid on the website).
pub const GROUPS: [&str; 12] = [
    "weapon", "shield", "tunic", "boots", "arrow", "spell", "tool", "upgrade", "song", "medallion",
    "stone", "key",
];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    pub id: String,
    pub group: String,
    pub age: Age,
    pub name_es: String,
    pub name_en: String,
    /// Two to four characters shown when the item has no icon.
    pub short: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub sort_order: i64,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogObjective {
    pub id: String,
    pub age: Age,
    pub name_es: String,
    pub name_en: String,
    pub sort_order: i64,
    /// Counts toward the default "required to finish" list when the event is created.
    pub required: bool,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub items: Vec<CatalogItem>,
    pub objectives: Vec<CatalogObjective>,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn text_ok(s: &str, max: usize) -> bool {
    let t = s.trim();
    !t.is_empty() && t.chars().count() <= max
}

impl CatalogItem {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("id must be 1-40 chars: lowercase letters, digits and dashes".into());
        }
        if !GROUPS.contains(&self.group.as_str()) {
            return Err(format!("group must be one of: {}", GROUPS.join(", ")));
        }
        if !text_ok(&self.name_es, 60) || !text_ok(&self.name_en, 60) {
            return Err("names (es/en) are required and limited to 60 characters".into());
        }
        if !text_ok(&self.short, 4) {
            return Err("short is required and limited to 4 characters".into());
        }
        if let Some(icon) = &self.icon
            && (icon.len() > 200 || !(icon.starts_with('/') || icon.starts_with("https://")))
        {
            return Err("icon must be a site path (/art/...) or an https URL".into());
        }
        Ok(())
    }
}

impl CatalogObjective {
    pub fn validate(&self) -> Result<(), String> {
        if !valid_id(&self.id) {
            return Err("id must be 1-40 chars: lowercase letters, digits and dashes".into());
        }
        if !text_ok(&self.name_es, 60) || !text_ok(&self.name_en, 60) {
            return Err("names (es/en) are required and limited to 60 characters".into());
        }
        Ok(())
    }
}

impl Catalog {
    /// Only what racers may report: disabled entries are treated as unknown.
    pub fn is_item(&self, id: &str) -> bool {
        self.items.iter().any(|i| i.enabled && i.id == id)
    }

    pub fn is_objective(&self, id: &str) -> bool {
        self.objectives.iter().any(|o| o.enabled && o.id == id)
    }

    pub fn objective_exists(&self, id: &str) -> bool {
        self.objectives.iter().any(|o| o.id == id)
    }

    /// The public view: enabled entries only, in display order.
    pub fn public(&self) -> Catalog {
        let mut items: Vec<_> = self.items.iter().filter(|i| i.enabled).cloned().collect();
        let mut objectives: Vec<_> = self
            .objectives
            .iter()
            .filter(|o| o.enabled)
            .cloned()
            .collect();
        items.sort_by(|a, b| (a.sort_order, &a.id).cmp(&(b.sort_order, &b.id)));
        objectives.sort_by(|a, b| (a.sort_order, &a.id).cmp(&(b.sort_order, &b.id)));
        Catalog { items, objectives }
    }

    /// The same catalog ordered by `sort_order` (how the database returns it).
    pub fn clone_sorted(&self) -> Catalog {
        let mut c = self.clone();
        c.items.sort_by(|a, b| (a.sort_order, &a.id).cmp(&(b.sort_order, &b.id)));
        c.objectives.sort_by(|a, b| (a.sort_order, &a.id).cmp(&(b.sort_order, &b.id)));
        c
    }

    /// Objectives that count toward finishing by default (used when the event is first created).
    pub fn default_required(&self) -> Vec<String> {
        let mut list: Vec<_> = self
            .objectives
            .iter()
            .filter(|o| o.enabled && o.required)
            .collect();
        list.sort_by_key(|o| o.sort_order);
        list.into_iter().map(|o| o.id.clone()).collect()
    }

    /// Short content hash: clients compare it to know whether their copy is current.
    pub fn version(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        let hash = Sha256::digest(json.as_bytes());
        hex::encode(&hash[..6])
    }
}

fn item(order: i64, id: &str, group: &str, age: Age, es: &str, en: &str, short: &str) -> CatalogItem {
    CatalogItem {
        id: id.into(),
        group: group.into(),
        age,
        name_es: es.into(),
        name_en: en.into(),
        short: short.into(),
        icon: None,
        sort_order: order,
        enabled: true,
    }
}

fn objective(order: i64, id: &str, age: Age, es: &str, en: &str) -> CatalogObjective {
    CatalogObjective {
        id: id.into(),
        age,
        name_es: es.into(),
        name_en: en.into(),
        sort_order: order,
        required: true,
        enabled: true,
    }
}

/// Factory catalog (Ocarina of Time). The first nine ids are the original ones and never change.
pub fn default_catalog() -> Catalog {
    use Age::{Adult, Both, Child};
    let items = vec![
        // --- Link niño -------------------------------------------------------------------------
        item(100, "kokiri-sword", "weapon", Child, "Espada Kokiri", "Kokiri Sword", "EK"),
        item(110, "deku-shield", "shield", Child, "Escudo Deku", "Deku Shield", "ED"),
        item(120, "fairy-slingshot", "tool", Child, "Tirachinas de Hada", "Fairy Slingshot", "TI"),
        item(130, "boomerang", "tool", Child, "Bumerán", "Boomerang", "BU"),
        item(140, "mask-of-truth", "tool", Child, "Máscara de la Verdad", "Mask of Truth", "MV"),
        item(150, "zeldas-letter", "key", Child, "Carta de Zelda", "Zelda's Letter", "CZ"),
        item(160, "kokiri-emerald", "stone", Child, "Esmeralda Kokiri", "Kokiri's Emerald", "EM"),
        item(170, "goron-ruby", "stone", Child, "Rubí Goron", "Goron's Ruby", "RG"),
        item(180, "zora-sapphire", "stone", Child, "Zafiro Zora", "Zora's Sapphire", "ZS"),
        // --- Link adulto -----------------------------------------------------------------------
        item(200, "master-sword", "weapon", Adult, "Espada Maestra", "Master Sword", "ME"),
        item(210, "biggoron-sword", "weapon", Adult, "Espada de Biggoron", "Biggoron's Sword", "EB"),
        item(220, "megaton-hammer", "weapon", Adult, "Martillo Megatón", "Megaton Hammer", "MM"),
        item(230, "mirror-shield", "shield", Adult, "Escudo Espejo", "Mirror Shield", "EE"),
        item(240, "goron-tunic", "tunic", Adult, "Túnica Goron", "Goron Tunic", "TG"),
        item(250, "zora-tunic", "tunic", Adult, "Túnica Zora", "Zora Tunic", "TZ"),
        item(260, "iron-boots", "boots", Adult, "Botas de Hierro", "Iron Boots", "BH"),
        item(270, "hover-boots", "boots", Adult, "Botas Aéreas", "Hover Boots", "BA"),
        item(280, "bow", "tool", Adult, "Arco de Hada", "Fairy Bow", "AR"),
        item(290, "hookshot", "tool", Adult, "Gancho", "Hookshot", "GA"),
        item(300, "longshot", "tool", Adult, "Gancho Largo", "Longshot", "GL"),
        item(310, "fire-arrows", "arrow", Adult, "Flechas de Fuego", "Fire Arrows", "FF"),
        item(320, "ice-arrows", "arrow", Adult, "Flechas de Hielo", "Ice Arrows", "FH"),
        item(330, "light-arrows", "arrow", Adult, "Flechas de Luz", "Light Arrows", "FL"),
        item(340, "silver-gauntlets", "upgrade", Adult, "Guanteletes de Plata", "Silver Gauntlets", "GP"),
        item(350, "golden-gauntlets", "upgrade", Adult, "Guanteletes de Oro", "Golden Gauntlets", "GO"),
        item(360, "gerudo-card", "key", Adult, "Tarjeta Gerudo", "Gerudo Membership Card", "TJ"),
        item(370, "forest-medallion", "medallion", Adult, "Medallón del Bosque", "Forest Medallion", "MB"),
        item(380, "fire-medallion", "medallion", Adult, "Medallón del Fuego", "Fire Medallion", "MF"),
        item(390, "water-medallion", "medallion", Adult, "Medallón del Agua", "Water Medallion", "MA"),
        item(400, "shadow-medallion", "medallion", Adult, "Medallón de las Sombras", "Shadow Medallion", "MS"),
        item(410, "spirit-medallion", "medallion", Adult, "Medallón del Espíritu", "Spirit Medallion", "MI"),
        item(420, "light-medallion", "medallion", Adult, "Medallón de la Luz", "Light Medallion", "ML"),
        // --- Ambos -----------------------------------------------------------------------------
        item(500, "bombs", "tool", Both, "Bombas", "Bombs", "BO"),
        item(510, "bombchus", "tool", Both, "Bombchus", "Bombchus", "BC"),
        item(520, "hylian-shield", "shield", Both, "Escudo Hyliano", "Hylian Shield", "EH"),
        item(530, "ocarina-of-time", "tool", Both, "Ocarina del Tiempo", "Ocarina of Time", "OT"),
        item(540, "lens-of-truth", "tool", Both, "Lente de la Verdad", "Lens of Truth", "LV"),
        item(550, "bottle", "tool", Both, "Botella", "Bottle", "BT"),
        item(560, "dins-fire", "spell", Both, "Fuego de Din", "Din's Fire", "FD"),
        item(570, "farores-wind", "spell", Both, "Viento de Farore", "Farore's Wind", "VF"),
        item(580, "nayrus-love", "spell", Both, "Amor de Nayru", "Nayru's Love", "AN"),
        item(590, "bomb-bag", "upgrade", Both, "Bolsa de Bombas", "Bomb Bag", "BB"),
        item(600, "wallet", "upgrade", Both, "Monedero", "Wallet", "MN"),
        item(610, "goron-bracelet", "upgrade", Both, "Brazalete Goron", "Goron's Bracelet", "BG"),
        item(620, "silver-scale", "upgrade", Both, "Escama de Plata", "Silver Scale", "EP"),
        item(630, "golden-scale", "upgrade", Both, "Escama de Oro", "Golden Scale", "EO"),
        item(640, "magic-meter", "upgrade", Both, "Medidor de Magia", "Magic Meter", "MG"),
        item(650, "double-defense", "upgrade", Both, "Doble Defensa", "Double Defense", "DD"),
        item(660, "stone-of-agony", "key", Both, "Piedra del Dolor", "Stone of Agony", "PD"),
        // --- Canciones (ambos) -----------------------------------------------------------------
        item(700, "zeldas-lullaby", "song", Both, "Nana de Zelda", "Zelda's Lullaby", "♪"),
        item(710, "eponas-song", "song", Both, "Canción de Epona", "Epona's Song", "♪"),
        item(720, "sarias-song", "song", Both, "Canción de Saria", "Saria's Song", "♪"),
        item(730, "suns-song", "song", Both, "Canción del Sol", "Sun's Song", "♪"),
        item(740, "song-of-time", "song", Both, "Canción del Tiempo", "Song of Time", "♪"),
        item(750, "song-of-storms", "song", Both, "Canción de la Tormenta", "Song of Storms", "♪"),
        item(760, "minuet-of-forest", "song", Both, "Minueto del Bosque", "Minuet of Forest", "♪"),
        item(770, "bolero-of-fire", "song", Both, "Bolero del Fuego", "Bolero of Fire", "♪"),
        item(780, "serenade-of-water", "song", Both, "Serenata del Agua", "Serenade of Water", "♪"),
        item(790, "requiem-of-spirit", "song", Both, "Réquiem del Espíritu", "Requiem of Spirit", "♪"),
        item(800, "nocturne-of-shadow", "song", Both, "Nocturno de las Sombras", "Nocturne of Shadow", "♪"),
        item(810, "prelude-of-light", "song", Both, "Preludio de la Luz", "Prelude of Light", "♪"),
    ];
    let objectives = vec![
        objective(10, "kokiri-forest", Child, "Bosque Kokiri", "Kokiri Forest"),
        objective(20, "deku-tree", Child, "Gran Árbol Deku", "Deku Tree"),
        objective(30, "dodongos-cavern", Child, "Cavernas Dodongo", "Dodongo's Cavern"),
        objective(40, "jabu-jabu", Child, "Vientre de Jabu-Jabu", "Jabu-Jabu's Belly"),
        objective(50, "forest-temple", Adult, "Templo del Bosque", "Forest Temple"),
        objective(60, "fire-temple", Adult, "Templo del Fuego", "Fire Temple"),
        objective(70, "water-temple", Adult, "Templo del Agua", "Water Temple"),
        objective(80, "shadow-temple", Adult, "Templo de las Sombras", "Shadow Temple"),
        objective(90, "spirit-temple", Adult, "Templo del Espíritu", "Spirit Temple"),
        objective(100, "ganons-castle", Adult, "Castillo de Ganon", "Ganon's Castle"),
    ];
    Catalog { items, objectives }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn default_catalog_is_valid_unique_and_keeps_the_original_ids() {
        let c = default_catalog();
        for i in &c.items {
            i.validate().unwrap_or_else(|e| panic!("{}: {e}", i.id));
        }
        for o in &c.objectives {
            o.validate().unwrap_or_else(|e| panic!("{}: {e}", o.id));
        }
        let ids: HashSet<_> = c.items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids.len(), c.items.len(), "duplicate item ids");
        let orders: HashSet<_> = c.items.iter().map(|i| i.sort_order).collect();
        assert_eq!(orders.len(), c.items.len(), "duplicate sort orders");
        for original in [
            "master-sword",
            "hookshot",
            "longshot",
            "bow",
            "bombs",
            "boomerang",
            "megaton-hammer",
            "iron-boots",
            "mirror-shield",
        ] {
            assert!(c.is_item(original), "{original} must stay");
        }
        assert!(c.items.len() >= 60);
        assert_eq!(c.objectives.len(), 10);
        assert_eq!(c.default_required().len(), 10);
    }

    #[test]
    fn every_age_is_represented_and_dungeons_split_between_child_and_adult() {
        let c = default_catalog();
        for age in [Age::Child, Age::Adult, Age::Both] {
            assert!(c.items.iter().any(|i| i.age == age));
        }
        let child: Vec<_> = c
            .objectives
            .iter()
            .filter(|o| o.age == Age::Child)
            .map(|o| o.id.as_str())
            .collect();
        assert_eq!(
            child,
            ["kokiri-forest", "deku-tree", "dodongos-cavern", "jabu-jabu"]
        );
    }

    #[test]
    fn disabled_entries_are_unknown_and_hidden_from_the_public_view() {
        let mut c = default_catalog();
        c.items.iter_mut().find(|i| i.id == "bow").unwrap().enabled = false;
        assert!(!c.is_item("bow"));
        assert!(!c.public().items.iter().any(|i| i.id == "bow"));
        assert!(c.is_item("hookshot"));
    }

    #[test]
    fn validation_rejects_bad_input() {
        let mut i = default_catalog().items[0].clone();
        i.id = "Bad Id".into();
        assert!(i.validate().is_err());
        let mut i = default_catalog().items[0].clone();
        i.group = "nope".into();
        assert!(i.validate().is_err());
        let mut i = default_catalog().items[0].clone();
        i.icon = Some("javascript:alert(1)".into());
        assert!(i.validate().is_err());
        i.icon = Some("/art/items/x.png".into());
        assert!(i.validate().is_ok());
    }

    #[test]
    fn version_changes_with_content() {
        let mut c = default_catalog();
        let v1 = c.version();
        assert_eq!(v1, c.version());
        c.items[0].name_en = "Renamed".into();
        assert_ne!(v1, c.version());
    }
}
