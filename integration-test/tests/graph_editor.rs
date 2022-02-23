use enso_integration_test::prelude::*;

use approx::assert_abs_diff_eq;
use enso_web::sleep;
use ensogl::display::navigation::navigator::ZoomEvent;
use std::time::Duration;
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
async fn zooming() {
    let test = IntegrationTestOnNewProject::setup().await;
    let project = test.project_view();
    let graph_editor = test.graph_editor();
    let camera = test.ide.ensogl_app.display.scene().layers.main.camera();
    let navigator = &graph_editor.model.navigator;

    let zoom_on_center = |amount: f32| ZoomEvent { focus: Vector2(0.0, 0.0), amount };
    let zoom_duration_ms = Duration::from_millis(1000);

    // Without debug mode
    navigator.emit_zoom_event(zoom_on_center(-1.0));
    sleep(zoom_duration_ms).await;
    assert_abs_diff_eq!(camera.zoom(), 1.0, epsilon = 0.001);
    navigator.emit_zoom_event(zoom_on_center(1.0));
    sleep(zoom_duration_ms).await;
    assert!(camera.zoom() < 1.0, "Camera zoom {} must be less than 1.0", camera.zoom());
    navigator.emit_zoom_event(zoom_on_center(-2.0));
    sleep(zoom_duration_ms).await;
    assert_abs_diff_eq!(camera.zoom(), 1.0, epsilon = 0.001);

    // With debug mode
    project.enable_debug_mode();
    navigator.emit_zoom_event(zoom_on_center(-1.0));
    sleep(zoom_duration_ms).await;
    assert!(camera.zoom() > 1.0, "Camera zoom {} must be greater than 1.0", camera.zoom());
    navigator.emit_zoom_event(zoom_on_center(5.0));
    sleep(zoom_duration_ms).await;
    assert!(camera.zoom() < 1.0, "Camera zoom {} must be less than 1.0", camera.zoom());
    navigator.emit_zoom_event(zoom_on_center(-5.0));
    sleep(zoom_duration_ms).await;
    assert!(camera.zoom() > 1.0, "Camera zoom {} must be greater than 1.0", camera.zoom());
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

    graph_editor.model.nodes.deselect_all();
    graph_editor.model.nodes.select(node_id);
    
    let node_added = graph_editor.node_added.next_event();
    add_node_button.click();

    let (node_id2, node_source) = node_added.expect();
    assert_eq!(node_source, Some(NodeSource{ node: node_id}));

    let camera = test.ide.ensogl_app.display.scene().layers.main.camera();

    camera.mod_position_xy(|pos| pos + Vector2(1000.0, 1000.0));

    graph_editor.model.nodes.deselect_all();
    let node_added = graph_editor.node_added.next_event();
    add_node_button.click();
    let (node_id3, node_source) = node_added.expect();
    
    let node_position = graph_editor.model.get_node_position(node_id3).expect("Node was not added");
    let center_of_screen = test.ide.ensogl_app.display.scene().screen_to_scene_coordinates(Vector3(0.0, 0.0, 0.0));
    assert_eq!(node_position.xy(), center_of_screen.xy());
}

