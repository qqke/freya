use dioxus::core::ElementId;
use dioxus_native_core::real_dom::NodeType;
use layers_engine::{Layers, NodeArea, NodeData};
use state::node::{DirectionMode, SizeMode};

fn calculate_area(node: &NodeData, mut area: NodeArea, parent_area: NodeArea) -> NodeArea {
    match node.width {
        SizeMode::Manual(w) => {
            area.width = w;
        }
        SizeMode::Percentage(per) => {
            area.width = ((parent_area.width as f32) / 100.0 * (per as f32)).round() as i32;
        }
        SizeMode::Auto => {}
    }

    match node.height {
        SizeMode::Manual(h) => {
            area.height = h;
        }
        SizeMode::Percentage(per) => {
            area.height = ((parent_area.height as f32) / 100.0 * (per as f32)).round() as i32;
        }
        SizeMode::Auto => {
            if let Some(node) = &node.node {
                if let NodeType::Element { tag, .. } = &node.node_type {
                    if tag == "text" {
                        area.height = 18;
                    }
                }
            }
        }
    }

    area
}

pub fn calculate_node<T>(
    node: &NodeData,
    remaining_area: NodeArea,
    parent_area: NodeArea,
    resolver_options: &mut T,
    layers: &mut Layers,
    node_resolver: fn(&ElementId, &mut T) -> Option<NodeData>,
    inherited_relative_layer: i16,
) -> NodeArea {
    let mut node_area = calculate_area(node, remaining_area, parent_area);
    let mut is_text = false;

    // Returns a tuple, the first element is the layer in which the current node must be added
    // and the second indicates the layer that it's children must inherit
    let (node_layer, inherited_relative_layer) =
        layers.calculate_layer(node, inherited_relative_layer);

    let padding = node.padding;
    let horizontal_padding = padding.1 + padding.3;
    let vertical_padding = padding.0 + padding.2;

    // Area that is available consideing the parent area
    let mut remaining_inner_area = NodeArea {
        x: node_area.x + padding.3,
        y: node_area.y + padding.0,
        width: node_area.width - horizontal_padding,
        height: node_area.height - vertical_padding,
    };
    // Visible area occupied by the child elements
    let inner_area = remaining_inner_area.clone();

    remaining_inner_area.y += node.node.as_ref().unwrap().state.size.scroll_y;
    remaining_inner_area.x += node.node.as_ref().unwrap().state.size.scroll_x;

    if let Some(dom_node) = &node.node {
        match &dom_node.node_type {
            NodeType::Element { children, .. } => {
                for child in children {
                    let child_node = node_resolver(child, resolver_options);

                    if let Some(child_node) = child_node {
                        let box_area = calculate_node::<T>(
                            &child_node,
                            remaining_inner_area,
                            inner_area,
                            resolver_options,
                            layers,
                            node_resolver,
                            inherited_relative_layer,
                        );

                        let state = &node.node.as_ref().unwrap().state;

                        if state.size.direction == DirectionMode::Vertical {
                            remaining_inner_area.y = box_area.y + box_area.height;
                            remaining_inner_area.height -= box_area.height;
                        } else {
                            remaining_inner_area.x = box_area.x + box_area.width;
                            remaining_inner_area.width -= box_area.width;
                        }

                        if box_area.width > remaining_inner_area.width
                            || remaining_inner_area.width == 0
                        {
                            remaining_inner_area.width = box_area.width;
                        }
                    }
                }
            }
            NodeType::Text { .. } => {
                is_text = true;
            }
            NodeType::Placeholder => {}
        }

        if !is_text {
            if let SizeMode::Auto = node.width {
                node_area.width = remaining_inner_area.x - node_area.x;
            }

            if let SizeMode::Auto = node.height {
                node_area.height = remaining_inner_area.y - node_area.y;
            }
        }
    }

    // Registers the element in the Layers handler
    layers.add_element(node, &node_area, node_layer);

    node_area
}
