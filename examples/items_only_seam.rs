use ambition_inventory_ui::{InventoryItemNode, InventorySlotId, ItemsOnlyPageSpec, MenuNode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoPage {
    Items,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DemoAction {
    Use(&'static str),
    ToggleEquip(&'static str),
}

fn main() {
    let page = ItemsOnlyPageSpec::new(DemoPage::Items, "Items")
        .selected_slot(Some(InventorySlotId(1)))
        .with_cell(
            InventoryItemNode::new(0, "Health Cell")
                .detail("Restores one heart")
                .count(3)
                .action(DemoAction::Use("health_cell")),
        )
        .with_cell(
            InventoryItemNode::new(1, "Axe")
                .detail("Held item")
                .equipped(true)
                .action(DemoAction::ToggleEquip("axe")),
        )
        .with_cell(InventoryItemNode::unowned(2, "Portal Gun"))
        .into_page_model();

    println!("{} page contains {} nodes", page.title, page.nodes.len());
    for node in page.actionable_nodes() {
        if let MenuNode::Control { label, action, .. } = node {
            println!("actionable: {label}: {action:?}");
        }
    }
}
