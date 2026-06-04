#[derive(Clone, Copy, Debug)]
struct MockItemSpec {
    name: &'static str,
    short: &'static str,
    glyph: &'static str,
    description: &'static str,
    kind: ItemKind,
    start_count: u32,
}

fn mock_items() -> &'static [MockItemSpec; ITEM_COUNT] {
    &MOCK_ITEMS
}

static MOCK_ITEMS: [MockItemSpec; ITEM_COUNT] = [
    MockItemSpec { name: "Portal Gun", short: "Portal", glyph: "O", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 1, description: "Held-item traversal tool: opens linked portals in the real Ambition game. This mock treats it as a held item, so it conflicts with Axe, Javelin, Bomb, and other held tools." },
    MockItemSpec { name: "Axe", short: "Axe", glyph: "A", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 1, description: "Held-item weapon: a heavy pirate axe. Equipping it replaces the current held item. Activating it again unequips it." },
    MockItemSpec { name: "Javelin", short: "Javelin", glyph: "J", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 1, description: "Held-item weapon: a throwable spear. It occupies the same one-object held slot as the Axe and Portal Gun." },
    MockItemSpec { name: "Gun-Sword", short: "GunSwrd", glyph: "G", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 1, description: "Held-item weapon: a laser sword with a gun on it that shoots swords. Mostly a stress-test for action labels and replacement messages." },
    MockItemSpec { name: "Puppy-Slug Gun", short: "SlugGun", glyph: "S", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 0, description: "Unowned held-item placeholder. It should remain visible for layout stability but disabled for host-side activation." },
    MockItemSpec { name: "Fireball", short: "Fire", glyph: "F", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 1, description: "Held spell verb. This may become an ability or spell resource in the real game, but the UI seam only needs to know that activating it requests a host action." },
    MockItemSpec { name: "Blink", short: "Blink", glyph: "B", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 1, description: "Held traversal verb. In this mock it competes for the same held slot as weapons and tools." },
    MockItemSpec { name: "Flight", short: "Fly", glyph: "Y", kind: ItemKind::Equippable(EquipSlot::Body), start_count: 1, description: "Body-slot movement mode. It conflicts with Morph Ball, Bubble Shield, and Debug Lens." },
    MockItemSpec { name: "Grapple Hook", short: "Grapple", glyph: "R", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 0, description: "Unowned held traversal tool placeholder." },
    MockItemSpec { name: "Morph Ball", short: "Morph", glyph: "M", kind: ItemKind::Equippable(EquipSlot::Body), start_count: 1, description: "Body-slot suit/mode. It conflicts with Flight and other body modes." },
    MockItemSpec { name: "Mark / Recall", short: "Recall", glyph: "K", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 0, description: "Held travel/combat tool placeholder; visible but not owned in this mock." },
    MockItemSpec { name: "Bubble Shield", short: "Bubble", glyph: "U", kind: ItemKind::Equippable(EquipSlot::Body), start_count: 1, description: "Body-slot defensive mode. Equipping it replaces Flight, Morph Ball, or Debug Lens." },
    MockItemSpec { name: "Health Cell", short: "Health", glyph: "H", kind: ItemKind::Consumable, start_count: 3, description: "Consumable: restores health in the real game. This mock decrements its count when activated." },
    MockItemSpec { name: "Mana Cell", short: "Mana", glyph: "N", kind: ItemKind::Consumable, start_count: 2, description: "Consumable: restores mana in the real game. Counts are host-owned and the renderer only displays them." },
    MockItemSpec { name: "Spare Battery", short: "Battery", glyph: "T", kind: ItemKind::Consumable, start_count: 2, description: "Consumable/resource placeholder used to demonstrate stack counts." },
    MockItemSpec { name: "Data Chip", short: "Chip", glyph: "D", kind: ItemKind::Consumable, start_count: 1, description: "Consumable/resource placeholder. Activating it decrements count to zero." },
    MockItemSpec { name: "Bomb", short: "Bomb", glyph: "Q", kind: ItemKind::Equippable(EquipSlot::HeldItem), start_count: 1, description: "Held explosive tool. This intentionally long description stress-tests the selected-item detail viewport. In the real Ambition menu, a bomb might explain blast radius, fuse timing, inventory limits, environmental interactions, and safety warnings. The important UI rule is that this text can be long without causing icon buttons, cube faces, equipment panels, or page dimensions to resize. The host can page or scroll this description while the Lunex cube remains geometrically stable." },
    MockItemSpec { name: "Gold Pouch", short: "Gold", glyph: "$", kind: ItemKind::Consumable, start_count: 2, description: "Consumable/resource placeholder." },
    MockItemSpec { name: "Map Fragment", short: "Map", glyph: "P", kind: ItemKind::KeyItem, start_count: 1, description: "Quest/key item. It is visible but has no direct action on the Items page." },
    MockItemSpec { name: "Sealed Note", short: "Note", glyph: "L", kind: ItemKind::KeyItem, start_count: 1, description: "Quest/key item. The mock host exposes it as display-only." },
    MockItemSpec { name: "Field Survey", short: "Survey", glyph: "V", kind: ItemKind::KeyItem, start_count: 0, description: "Unowned key-item placeholder." },
    MockItemSpec { name: "Gate Key", short: "Gate", glyph: "E", kind: ItemKind::KeyItem, start_count: 1, description: "Quest/key item. This would unlock a gate in the real game, but the inventory UI should not own that effect." },
    MockItemSpec { name: "Debug Lens", short: "Lens", glyph: "I", kind: ItemKind::Equippable(EquipSlot::Body), start_count: 1, description: "Body-slot inspection mode in this mock." },
    MockItemSpec { name: "Reserved", short: "--", glyph: ".", kind: ItemKind::Reserved, start_count: 0, description: "Reserved for a future item." },
];
