#[derive(Clone, Copy)]
struct OotItem { name: &'static str, _short: &'static str, icon: &'static str, detail: Option<&'static str>, important: bool, adult_usable: bool }
impl OotItem {
    fn usable_by_current_link(self) -> bool {
        if LINK_IS_ADULT { self.adult_usable } else { true }
    }
}
fn oot_items() -> [OotItem; 24] {
    // Source-like inventory slot order from OoT's InventorySlot enum:
    // row 1: sticks/nuts/bombs/bow/fire/din
    // row 2: slingshot/ocarina/bombchu/hookshot/ice/farore
    // row 3: boomerang/lens/beans/hammer/light/nayru
    // row 4: bottle1..4/adult trade/child trade
    [
        OotItem { name: "Deku Stick", _short: "Stick", icon: "icons/oot/deku_stick.png", detail: Some("x99"), important: false , adult_usable: false },
        OotItem { name: "Deku Nut", _short: "Nut", icon: "icons/oot/deku_nut.png", detail: Some("x99"), important: false , adult_usable: true },
        OotItem { name: "Bomb", _short: "Bomb", icon: "icons/oot/bomb.png", detail: Some("x99"), important: false , adult_usable: true },
        OotItem { name: "Fairy Bow", _short: "Bow", icon: "icons/oot/bow.png", detail: Some("x50"), important: true , adult_usable: true },
        OotItem { name: "Fire Arrow", _short: "Fire", icon: "icons/oot/fire_arrow.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Din's Fire", _short: "Din", icon: "icons/oot/dins_fire.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Fairy Slingshot", _short: "Shot", icon: "icons/oot/slingshot.png", detail: Some("x50"), important: true , adult_usable: false },
        OotItem { name: "Ocarina of Time", _short: "Ocarina", icon: "icons/oot/ocarina.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Bombchu", _short: "Bombchu", icon: "icons/oot/bombchu.png", detail: Some("x50"), important: false , adult_usable: true },
        OotItem { name: "Longshot", _short: "Long", icon: "icons/oot/longshot.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Ice Arrow", _short: "Ice", icon: "icons/oot/ice_arrow.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Farore's Wind", _short: "Farore", icon: "icons/oot/farores_wind.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Boomerang", _short: "Boom", icon: "icons/oot/boomerang.png", detail: None, important: true , adult_usable: false },
        OotItem { name: "Lens of Truth", _short: "Lens", icon: "icons/oot/lens.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Magic Bean", _short: "Bean", icon: "icons/oot/beans.png", detail: Some("x10"), important: false , adult_usable: false },
        OotItem { name: "Megaton Hammer", _short: "Hammer", icon: "icons/oot/hammer.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Light Arrow", _short: "Light", icon: "icons/oot/light_arrow.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Nayru's Love", _short: "Nayru", icon: "icons/oot/nayrus_love.png", detail: None, important: true , adult_usable: true },
        OotItem { name: "Bottle", _short: "Fairy", icon: "icons/oot/bottle.png", detail: Some("Fairy"), important: true , adult_usable: true },
        OotItem { name: "Bottle", _short: "Milk", icon: "icons/oot/milk.png", detail: Some("Milk"), important: true , adult_usable: true },
        OotItem { name: "Bottle", _short: "Fire", icon: "icons/oot/bottle.png", detail: Some("Fire"), important: true , adult_usable: true },
        OotItem { name: "Bottle", _short: "Poe", icon: "icons/oot/poe.png", detail: Some("Poe"), important: true , adult_usable: true },
        OotItem { name: "Claim Check", _short: "Check", icon: "icons/oot/claim_check.png", detail: None, important: false , adult_usable: true },
        OotItem { name: "Mask", _short: "Mask", icon: "icons/oot/mask.png", detail: None, important: false , adult_usable: false },
    ]
}
#[derive(Clone, Copy)]
struct EquipChoice { name: &'static str, _short: &'static str, icon: &'static str, adult_usable: bool }
impl EquipChoice {
    fn usable_by_current_link(self) -> bool {
        if LINK_IS_ADULT { self.adult_usable } else { true }
    }
}
#[derive(Clone, Copy)]
struct EquipSlot { name: &'static str, choices: [EquipChoice; 3] }
fn equip_slots() -> [EquipSlot; 4] {
    [
        EquipSlot { name: "Sword", choices: [
            EquipChoice { name: "Kokiri Sword", _short: "Kok", icon: "icons/oot/kokiri_sword.png", adult_usable: false },
            EquipChoice { name: "Master Sword", _short: "Mas", icon: "icons/oot/master_sword.png", adult_usable: true },
            EquipChoice { name: "Biggoron Sword", _short: "Big", icon: "icons/oot/biggoron_sword.png", adult_usable: true },
        ]},
        EquipSlot { name: "Shield", choices: [
            EquipChoice { name: "Deku Shield", _short: "Deku", icon: "icons/oot/deku_shield.png", adult_usable: false },
            EquipChoice { name: "Hylian Shield", _short: "Hyl", icon: "icons/oot/hylian_shield.png", adult_usable: true },
            EquipChoice { name: "Mirror Shield", _short: "Mir", icon: "icons/oot/mirror_shield.png", adult_usable: true },
        ]},
        EquipSlot { name: "Tunic", choices: [
            EquipChoice { name: "Kokiri Tunic", _short: "Kok", icon: "icons/oot/kokiri_tunic.png", adult_usable: true },
            EquipChoice { name: "Goron Tunic", _short: "Gor", icon: "icons/oot/goron_tunic.png", adult_usable: true },
            EquipChoice { name: "Zora Tunic", _short: "Zora", icon: "icons/oot/zora_tunic.png", adult_usable: true },
        ]},
        EquipSlot { name: "Boots", choices: [
            EquipChoice { name: "Kokiri Boots", _short: "Kok", icon: "icons/oot/kokiri_boots.png", adult_usable: true },
            EquipChoice { name: "Iron Boots", _short: "Iron", icon: "icons/oot/iron_boots.png", adult_usable: true },
            EquipChoice { name: "Hover Boots", _short: "Hover", icon: "icons/oot/hover_boots.png", adult_usable: true },
        ]},
    ]
}

#[derive(Clone, Copy)]
struct MapMarker { name: &'static str, short: &'static str, x: f32, y: f32 }
fn map_markers() -> [MapMarker; 8] {
    [
        MapMarker { name: "Kokiri Forest", short: "K", x: 63.0, y: 55.0 },
        MapMarker { name: "Lost Woods", short: "W", x: 57.0, y: 46.0 },
        MapMarker { name: "Market", short: "M", x: 50.0, y: 35.0 },
        MapMarker { name: "Death Mountain", short: "D", x: 59.0, y: 28.0 },
        MapMarker { name: "Zora Domain", short: "Z", x: 67.0, y: 42.0 },
        MapMarker { name: "Lake Hylia", short: "L", x: 40.0, y: 61.0 },
        MapMarker { name: "Gerudo Valley", short: "G", x: 28.0, y: 48.0 },
        MapMarker { name: "Lon Lon Ranch", short: "R", x: 47.0, y: 50.0 },
    ]
}

#[derive(Clone, Copy)]
struct QuestIcon { name: &'static str, _short: &'static str, icon: &'static str }
fn quest_icons() -> [QuestIcon; 6] {
    [
        QuestIcon { name: "Forest Medallion", _short: "Fo", icon: "icons/oot/med_forest.png" },
        QuestIcon { name: "Fire Medallion", _short: "Fi", icon: "icons/oot/med_fire.png" },
        QuestIcon { name: "Water Medallion", _short: "Wa", icon: "icons/oot/med_water.png" },
        QuestIcon { name: "Spirit Medallion", _short: "Sp", icon: "icons/oot/med_spirit.png" },
        QuestIcon { name: "Shadow Medallion", _short: "Sh", icon: "icons/oot/med_shadow.png" },
        QuestIcon { name: "Light Medallion", _short: "Li", icon: "icons/oot/med_light.png" },
    ]
}
fn stones() -> [QuestIcon; 3] {
    [
        QuestIcon { name: "Kokiri Emerald", _short: "Em", icon: "icons/oot/stone_emerald.png" },
        QuestIcon { name: "Goron Ruby", _short: "Ru", icon: "icons/oot/stone_ruby.png" },
        QuestIcon { name: "Zora Sapphire", _short: "Sa", icon: "icons/oot/stone_sapphire.png" },
    ]
}


fn all_quest_icons() -> Vec<QuestIcon> {
    let mut out = Vec::new();
    out.extend_from_slice(&quest_icons());
    out.extend_from_slice(&stones());
    out
}

#[derive(Clone, Copy)]
struct Song { name: &'static str, _short: &'static str, icon: &'static str, pattern: &'static str }
fn songs() -> [Song; 12] {
    [
        Song { name: "Minuet of Forest", _short: "Min", icon: "icons/oot/song_minuet.png", pattern: "A ↑ ← → ← →" },
        Song { name: "Bolero of Fire", _short: "Bol", icon: "icons/oot/song_bolero.png", pattern: "↓ A ↓ A → ↓ → ↓" },
        Song { name: "Serenade of Water", _short: "Ser", icon: "icons/oot/song_serenade.png", pattern: "A ↓ → → ←" },
        Song { name: "Requiem of Spirit", _short: "Req", icon: "icons/oot/song_requiem.png", pattern: "A ↓ A → ↓ A" },
        Song { name: "Nocturne of Shadow", _short: "Noc", icon: "icons/oot/song_nocturne.png", pattern: "← → → A ← → ↓" },
        Song { name: "Prelude of Light", _short: "Pre", icon: "icons/oot/song_prelude.png", pattern: "↑ → ↑ → ← ↑" },
        Song { name: "Zelda's Lullaby", _short: "Zel", icon: "icons/oot/song_lullaby.png", pattern: "← ↑ → ← ↑ →" },
        Song { name: "Epona's Song", _short: "Epo", icon: "icons/oot/song_epona.png", pattern: "↑ ← → ↑ ← →" },
        Song { name: "Saria's Song", _short: "Sar", icon: "icons/oot/song_saria.png", pattern: "↓ → ← ↓ → ←" },
        Song { name: "Sun's Song", _short: "Sun", icon: "icons/oot/song_sun.png", pattern: "→ ↓ ↑ → ↓ ↑" },
        Song { name: "Song of Time", _short: "Tim", icon: "icons/oot/song_time.png", pattern: "→ A ↓ → A ↓" },
        Song { name: "Song of Storms", _short: "Sto", icon: "icons/oot/song_storms.png", pattern: "A ↓ ↑ A ↓ ↑" },
    ]
}
