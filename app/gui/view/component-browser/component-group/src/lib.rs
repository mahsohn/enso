//! This module defines a widget for displaying a list of entries of a component group and the name
//! of the component group.
//!
//! The widget is defined by the [`View`].
//!
//! To learn more about component groups, see the [Component Browser Design
//! Document](https://github.com/enso-org/design/blob/e6cffec2dd6d16688164f04a4ef0d9dff998c3e7/epics/component-browser/design.md).

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

use ensogl_core::display::shape::*;
use ensogl_core::prelude::*;

use enso_frp as frp;
use ensogl_core::application::Application;
use ensogl_core::data::color::Rgba;
use ensogl_core::display;
use ensogl_gui_component::component;
use ensogl_hardcoded_theme::application::component_browser::component_group as theme;
use ensogl_list_view as list_view;
use ensogl_text as text;



// =================
// === Constants ===
// =================

const HEADER_FONT: &str = "DejaVuSans-Bold";



// ==========================
// === Shapes Definitions ===
// ==========================


// === Background ===

/// The background of the [`View`].
pub mod background {
    use super::*;

    ensogl_core::define_shape_system! {
        below = [list_view::background];
        (style:Style, color:Vector4) {
            let sprite_width: Var<Pixels> = "input_size.x".into();
            let sprite_height: Var<Pixels> = "input_size.y".into();
            let color = Var::<Rgba>::from(color);
            // TODO[MC,WD]: We should use Plane here, but it has a bug - renders wrong color. See:
            //   https://github.com/enso-org/enso/pull/3373#discussion_r849054476
            let shape = Rect((&sprite_width, &sprite_height)).fill(color);
            shape.into()
        }
    }
}

/// The background of the header.
pub mod header_background {
    use super::*;

    ensogl_core::define_shape_system! {
        above = [background, list_view::background];
        (style:Style, color:Vector4) {
            let sprite_width: Var<Pixels> = "input_size.x".into();
            let sprite_height: Var<Pixels> = "input_size.y".into();
            let color = Var::<Rgba>::from(color);
            // TODO[MC,WD]: We should use Plane here, but it has a bug - renders wrong color. See:
            //   https://github.com/enso-org/enso/pull/3373#discussion_r849054476
            let shape = Rect((&sprite_width, &sprite_height)).fill(color);
            shape.into()
        }
    }
}


// =======================
// === Header Geometry ===
// =======================

#[derive(Debug, Copy, Clone, Default)]
struct HeaderGeometry {
    height:         f32,
    padding_left:   f32,
    padding_right:  f32,
    padding_bottom: f32,
}

impl HeaderGeometry {
    fn from_style(style: &StyleWatchFrp, network: &frp::Network) -> frp::Sampler<Self> {
        let height = style.get_number(theme::header::height);
        let padding_left = style.get_number(theme::header::padding::left);
        let padding_right = style.get_number(theme::header::padding::right);
        let padding_bottom = style.get_number(theme::header::padding::bottom);

        frp::extend! { network
            init <- source_();
            theme <- all_with5(&init,&height,&padding_left,&padding_right,&padding_bottom,
                |_,&height,&padding_left,&padding_right,&padding_bottom|
                    Self{height,padding_left,padding_right,padding_bottom}
            );
            theme_sampler <- theme.sampler();
        }
        init.emit(());
        theme_sampler
    }
}



// ===========
// === FRP ===
// ===========

ensogl_core::define_endpoints_2! {
    Input {
        set_header(String),
        set_entries(list_view::entry::AnyModelProvider<list_view::entry::Label>),
        set_background_color(Rgba),
        set_size(Vector2),
        set_viewport_size(f32),
    }
    Output {}
}

impl component::Frp<Model> for Frp {
    fn init(api: &Self::Private, _app: &Application, model: &Model, style: &StyleWatchFrp) {
        let network = &api.network;
        let input = &api.input;
        let header_text_size = style.get_number(theme::header::text::size);
        frp::extend! { network

            // === Geometry ===

            let header_geometry = HeaderGeometry::from_style(style, network);
            size_and_header_geometry <- all3(&input.set_size, &header_geometry, &input.set_viewport_size);
            eval size_and_header_geometry(((size, hdr_geom, vprt_size)) model.resize(*size, *hdr_geom, *vprt_size));


            // === Header ===

            init <- source_();
            header_text_size <- all(&header_text_size, &init)._0();
            model.header.set_default_text_size <+ header_text_size.map(|v| text::Size(*v));
            _set_header <- input.set_header.map2(&size_and_header_geometry, f!(
                (text, (size, hdr_geom,_)) {
                    model.header_text.replace(text.clone());
                    model.update_header_width(*size, *hdr_geom);
                })
            );
            eval input.set_background_color((c)
                model.background.color.set(c.into()));
            eval input.set_background_color((c)
                model.header_background.color.set(c.into()));


            // === Entries ===

            model.entries.set_background_color(HOVER_COLOR);
            model.entries.show_background_shadow(false);
            model.entries.set_background_corners_radius(0.0);
            model.entries.set_background_color <+ input.set_background_color;
            model.entries.set_entries <+ input.set_entries;
        }
        init.emit(());
    }
}



// =============
// === Model ===
// =============

/// The Model of the [`View`] component.
#[derive(Clone, CloneRef, Debug)]
pub struct Model {
    display_object:    display::object::InstanceWithLayer<display::scene::layer::Layer>,
    header:            text::Area,
    header_background: header_background::View,
    header_text:       Rc<RefCell<String>>,
    background:        background::View,
    entries:           list_view::ListView<list_view::entry::Label>,
    text_layer:        display::scene::layer::Layer,
    header_layer:      display::scene::layer::Layer,
    header_text_layer: display::scene::layer::Layer,
}

impl display::Object for Model {
    fn display_object(&self) -> &display::object::Instance {
        &self.display_object
    }
}

impl component::Model for Model {
    fn label() -> &'static str {
        "ComponentGroup"
    }

    fn new(app: &Application, logger: &Logger) -> Self {
        let header_text = default();
        let display_object = display::object::Instance::new(&logger);
        use display::scene::layer::Layer;
        let cgv_layer = Layer::new_with_cam(
            logger.clone_ref(),
            &app.display.default_scene.layers.main.camera(),
        );
        let text_layer = display::scene::layer::Layer::new_with_cam(
            logger.clone_ref(),
            &app.display.default_scene.layers.main.camera(),
        );
        let header_layer = display::scene::layer::Layer::new_with_cam(
            logger.clone_ref(),
            &app.display.default_scene.layers.main.camera(),
        );
        let header_text_layer = Layer::new_with_cam(logger.clone_ref(), &header_layer.camera());
        header_layer.add_sublayer(&header_text_layer);
        text_layer.add_sublayer(&header_layer);
        cgv_layer.add_sublayer(&text_layer);
        let display_object = display::object::InstanceWithLayer::new(display_object, cgv_layer);
        let background = background::View::new(&logger);
        display_object.layer.add_exclusive(&background);
        let header_background = header_background::View::new(&logger);
        header_layer.add_exclusive(&header_background);
        let header = text::Area::new(app);
        header.add_to_scene_layer(&header_text_layer);
        let entries = list_view::ListView::new(app);
        display_object.layer.add_exclusive(&entries);
        entries.set_label_layer2(text_layer.clone_ref());
        display_object.add_child(&background);
        display_object.add_child(&header_background);
        display_object.add_child(&header);
        display_object.add_child(&entries);

        header.set_font(HEADER_FONT);

        Model { display_object, text_layer, header_layer, header_text_layer, header, header_text, background, header_background, entries }
    }
}

impl Model {
    fn resize(&self, size: Vector2, header_geometry: HeaderGeometry, viewport_height: f32) {
        // === Background ===

        self.background.size.set(size);


        // === Header Text ===
        let header_height = self.update_header_height(size, header_geometry, viewport_height);

        // === Entries ===

        self.entries.resize(size - Vector2(0.0, header_height));
        self.entries.set_position_y(-header_height / 2.0);
    }

    fn update_header_height(
        &self,
        size: Vector2,
        header_geometry: HeaderGeometry,
        viewport_height: f32,
    ) -> f32 {
        let header_padding_left = header_geometry.padding_left;
        let header_text_x = -size.x / 2.0 + header_padding_left;
        let header_text_height = self.header.height.value();
        let header_padding_bottom = header_geometry.padding_bottom;
        let header_height = header_geometry.height;
        let header_bottom_y = size.y / 2.0 - header_height;
        let header_text_y =
            header_bottom_y + header_text_height + header_padding_bottom - viewport_height;
        let header_text_y = header_text_y.max(-size.y / 2.0);
        self.header.set_position_xy(Vector2(header_text_x, header_text_y));
        self.update_header_width(size, header_geometry);
        self.header_background.size.set(Vector2(size.x, header_height));
        self.header_background.set_position_xy(Vector2(
            header_text_x + size.x / 2.0 - header_padding_left,
            header_text_y,
        ));
        header_height
    }

    fn update_header_width(&self, size: Vector2, header_geometry: HeaderGeometry) {
        let header_padding_left = header_geometry.padding_left;
        let header_padding_right = header_geometry.padding_right;
        let max_text_width = size.x - header_padding_left - header_padding_right;
        self.header.set_content_truncated(self.header_text.borrow().clone(), max_text_width);
    }
}



// ============
// === View ===
// ============

/// A widget for displaying the entries and name of a component group.
///
/// The widget is rendered as a header label, a list of entries below it, and a colored background.
///
/// To learn more about component groups, see the [Component Browser Design
/// Document](https://github.com/enso-org/design/blob/e6cffec2dd6d16688164f04a4ef0d9dff998c3e7/epics/component-browser/design.md).
pub type View = component::ComponentView<Model, Frp>;
