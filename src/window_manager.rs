use swayipc::Connection;

pub fn visible_windows() -> Vec<String> {
    let mut conn = Connection::new().unwrap();
    let mut res = Vec::new();

    let root = conn.get_tree().unwrap();
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

    res
}
