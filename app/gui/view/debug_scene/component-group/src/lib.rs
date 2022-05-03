//! A debug scene which shows the Component Group visual component.

// === Standard Linter Configuration ===
#![deny(non_ascii_idents)]
#![warn(unsafe_code)]
// === Non-Standard Linter Configuration ===
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]

use ensogl_core::prelude::*;
use wasm_bindgen::prelude::*;

use ensogl_core::application::Application;
use ensogl_core::data::color;
use ensogl_core::display::object::ObjectOps;
use ensogl_core::display::shape::*;
use ensogl_hardcoded_theme as theme;
use ensogl_list_view as list_view;
use ensogl_text_msdf_sys::run_once_initialized;
use ide_view_component_group as component_group;
use ensogl_scroll_area::ScrollArea;
use ensogl_scrollbar::Scrollbar;
use enso_frp as frp;



// ===================
// === Entry Point ===
// ===================

/// An entry point.
#[entry_point]
pub fn main() {
    run_once_initialized(|| {
        let app = Application::new("root");
        init(&app);
        mem::forget(app);
    });
}



// ====================
// === Mock Entries ===
// ====================

#[derive(Clone, Debug)]
struct MockEntries {
    entries: Vec<String>,
}

impl MockEntries {
    fn new(entries: Vec<String>) -> Self {
        Self { entries }
    }

    fn get_entry(&self, i: usize) -> Option<String> {
        self.entries.get(i).cloned()
    }
}

impl list_view::entry::ModelProvider<list_view::entry::Label> for MockEntries {
    fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn get(&self, id: usize) -> Option<String> {
        self.get_entry(id)
    }
}

mod background {
    use super::*;
    ensogl_core::define_shape_system! {
        (style:Style) {
            let width: Var<Pixels> = "input_size.x".into();
            let height: Var<Pixels> = "input_size.y".into();
            let color = color::Rgb::new(0.9, 0.9, 0.9);
            Rect((width, height)).fill(color).into()
        }
    }
}


// ========================
// === Init Application ===
// ========================

fn init(app: &Application) {
    theme::builtin::dark::register(&app);
    theme::builtin::light::register(&app);
    theme::builtin::light::enable(&app);

    let mock_entries = MockEntries::new(vec![
        "long sample entry with text overflowing the width".into(),
        "convert".into(),
        "table input".into(),
        "text input".into(),
        "number input".into(),
        "table input".into(),
        "data output".into(),
        "data input".into(),
    ]);

    let scroll_bar = Scrollbar::new(app);
    scroll_bar.set_length(400.0);
    scroll_bar.set_max(400.0);
    scroll_bar.set_thumb_size(20.0);
    scroll_bar.set_rotation_z(-90.0_f32.to_radians());
    scroll_bar.set_position_x(-10.0);
    app.display.add_child(&scroll_bar);

    let component_group = app.new_view::<component_group::View>();
    let provider = list_view::entry::AnyModelProvider::new(mock_entries);
    let group_name = "Long group name with text overflowing the width";
    component_group.set_header(group_name.to_string());
    component_group.set_entries(provider);
    component_group.set_size(Vector2(150.0, 200.0));
    component_group.set_position_xy(Vector2(75.0, 0.0));
    component_group.set_background_color(color::Rgba(0.927, 0.937, 0.913, 1.0));
    app.display.add_child(&component_group);

    let network = frp::Network::new("network");
    frp::extend!{ network
        eval scroll_bar.output.thumb_position ((y) component_group.set_viewport_size(*y));
        eval scroll_bar.output.thumb_position ((y) component_group.set_position_y(*y));
    }

    std::mem::forget(component_group);
    std::mem::forget(scroll_bar);
    std::mem::forget(network);
}
