use color_eyre::eyre::Context;
use color_eyre::Section;
use swayipc::Connection;

pub fn visible_windows() -> color_eyre::Result<Vec<String>> {
    let mut conn = Connection::new()
        .wrap_err("Could not connect to sway")
        .note(
        "The skip-when-visible option only works with the Sway window manager",
    )?;
    let mut res = Vec::new();

    let root = conn
        .get_tree()
        .wrap_err("Error getting window tree from Sway")?;
    let mut nodes = root.nodes;
    while !nodes.is_empty() {
        if let Some(node) = nodes.pop() {
            nodes.extend(node.nodes);
            if node.visible.is_some_and(|v| v) {
                if let Some(name) = node.name {
                    res.push(name)
                }
            }
        }
    }

    Ok(res)
}
