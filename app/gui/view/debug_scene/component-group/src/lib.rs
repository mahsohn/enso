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

mod green_circle {
    use super::*;
    ensogl_core::define_shape_system! {
        (style:Style) {
            Circle(70.px()).fill(color::Rgba::transparent()).into()
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

    let scroll_area = ScrollArea::new(app);
    scroll_area.set_position_xy(Vector2(-75.0, 100.0));
    scroll_area.resize(Vector2(250.0, 400.0));
    scroll_area.set_content_width(250.0);
    scroll_area.set_content_height(1000.0);
    app.display.add_child(&scroll_area);

    let green_circle = green_circle::View::new(&app.logger);
    green_circle.size.set(Vector2(150.0, 150.0));
    green_circle.set_position_xy(Vector2(200.0, -150.0));
    scroll_area.content().add_child(&green_circle);

    let component_group = app.new_view::<component_group::View>();
    let provider = list_view::entry::AnyModelProvider::new(mock_entries);
    let group_name = "Long group name with text overflowing the width";
    component_group.set_header(group_name.to_string());
    component_group.set_entries(provider);
    component_group.set_size(Vector2(150.0, 200.0));
    component_group.set_position_xy(Vector2(75.0, -100.0));
    component_group.set_background_color(color::Rgba(0.927, 0.937, 0.913, 1.0));
    scroll_area.content().add_child(&component_group);

    let network = frp::Network::new("network");
    frp::extend!{ network
        eval scroll_area.output.scroll_position_y ((y) component_group.set_viewport_size(*y));
    }

    std::mem::forget(component_group);
    std::mem::forget(scroll_area);
    std::mem::forget(green_circle);
    std::mem::forget(network);
}
