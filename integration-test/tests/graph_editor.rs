use enso_integration_test::prelude::*;

use ordered_float::OrderedFloat;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn create_new_project_and_add_nodes() {
    let test = IntegrationTestOnNewProject::setup().await;
    let graph_editor = test.graph_editor();

    assert_eq!(graph_editor.model.nodes.all.len(), 2);
    let expect_node_added = graph_editor.node_added.next_event();
    graph_editor.add_node();
    let (added_node_id, source_node) = expect_node_added.expect();
    assert_eq!(source_node, None);
    assert_eq!(graph_editor.model.nodes.all.len(), 3);

    let added_node =
        graph_editor.model.nodes.get_cloned_ref(&added_node_id).expect("Added node is not added");
    assert_eq!(added_node.view.expression.value().to_string(), "");
}

#[wasm_bindgen_test]
async fn debug_mode() {
    let test = IntegrationTestOnNewProject::setup().await;
    let project = test.project_view();
    let graph_editor = test.graph_editor();

    assert!(!graph_editor.debug_mode.value());

    // Turning On
    let expect_mode = project.debug_mode.next_event();
    let expect_popup_message = project.debug_mode_popup().label().show.next_event();
    project.enable_debug_mode();
    assert!(expect_mode.expect());
    let message = expect_popup_message.expect();
    assert!(
        message.contains("Debug Mode enabled"),
        "Message \"{}\" does not mention enabling Debug mode",
        message
    );
    assert!(
        message.contains(enso_gui::view::debug_mode_popup::DEBUG_MODE_SHORTCUT),
        "Message \"{}\" does not inform about shortcut to turn mode off",
        message
    );
    assert!(graph_editor.debug_mode.value());

    // Turning Off
    let expect_mode = project.debug_mode.next_event();
    let expect_popup_message = project.debug_mode_popup().label().show.next_event();
    project.disable_debug_mode();
    assert!(!expect_mode.expect());
    let message = expect_popup_message.expect();
    assert!(
        message.contains("Debug Mode disabled"),
        "Message \"{}\" does not mention disabling of debug mode",
        message
    );
    assert!(!graph_editor.debug_mode.value());
}

#[wasm_bindgen_test]
async fn adding_node_with_add_node_button() {
    let test = IntegrationTestOnNewProject::setup().await;
    let project = test.project_view();
    let graph_editor = test.graph_editor();

    let add_node_button = &graph_editor.model.add_node_button;
    let node_added = graph_editor.node_added.next_event();
    
    let nodes_and_positions = graph_editor.model.nodes.all.keys().into_iter().flat_map(|id| graph_editor.model.get_node_position(id).map(|pos| (id, pos)));
    let mut nodes_sorted_by_y_axis = nodes_and_positions.sorted_by_key(|(_, pos)| OrderedFloat(pos.y));
    let (_, bottom_most_pos) = nodes_sorted_by_y_axis.next().expect("Default project does not contain any nodes");

    add_node_button.click();

    let (node_id, node_source) = node_added.expect();
    assert!(node_source.is_none());
    let node_position = graph_editor.model.get_node_position(node_id).expect("Node was not added");
    // Node is created below the bottom-most one
    assert!(node_position.y < bottom_most_pos.y, "Expected that {node_position} < {bottom_most_pos}");

    assert_eq!(graph_editor.model.nodes.last_selected(), Some(node_id));
}
