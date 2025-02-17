//! This module defines a widget for displaying a list of entries of a component group and the name
//! of the component group.
//!
//! The widget is defined by the [`View`].
//!
//! To learn more about component groups, see the [Component Browser Design
//! Document](https://github.com/enso-org/design/blob/e6cffec2dd6d16688164f04a4ef0d9dff998c3e7/epics/component-browser/design.md).
//!
//! # Header and its shadow
//!
//! To simulate scrolling of the component group entries we move the header of the component group
//! down while moving the whole component group up (see [`Frp::set_header_pos`]). When the header
//! is moved down the shadow appears below it. The shadow changes its intensity smoothly before
//! the header reaches the [`HEADER_SHADOW_PEAK`] distance from the top of the component group.
//! After that the shadow is unchanged. When the header approaches the bottom of the component group
//! we gradually reduce the size of the shadow so that it will never be rendered outside the
//! component group boundaries. See `Header Backgound` section in the [`Model::resize`] method.

#![recursion_limit = "512"]
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

use ensogl_core::application::traits::*;
use ensogl_core::display::shape::*;
use ensogl_core::prelude::*;

use enso_frp as frp;
use ensogl_core::application::shortcut::Shortcut;
use ensogl_core::application::Application;
use ensogl_core::data::color;
use ensogl_core::display;
use ensogl_core::display::camera::Camera2d;
use ensogl_core::display::scene::layer;
use ensogl_gui_component::component;
use ensogl_hardcoded_theme::application::component_browser::component_group as theme;
use ensogl_list_view as list_view;
use ensogl_shadow as shadow;
use ensogl_text as text;


// ==============
// === Export ===
// ==============

pub mod entry;
pub mod wide;

pub use entry::View as Entry;



// =================
// === Constants ===
// =================

/// The distance (in pixels) from the top border of the component group at which
/// the header shadow reaches its maximum intensity.
///
/// When the header is on top of the component group (default position) the shadow is invisible.
/// Once the header moves down (see [`Frp::set_header_pos`]) we start to linearly increase the
/// intensity of the shadow until it reaches its maximum at [`HEADER_SHADOW_PEAK`] distance from
/// the top. The intensity does not change all the way down from there, but the shadow is clipped at
/// the bottom of the component group so that it will never escape the borders of the group and
/// will not cover any neighboring elements of the scene. (see [`Model::resize`] method)
const HEADER_SHADOW_PEAK: f32 = list_view::entry::HEIGHT / 2.0;



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
            let color = Var::<color::Rgba>::from(color);
            Plane().fill(color).into()
        }
    }
}


// === Header Background ===

/// The background of the header. It consists of a rectangle that matches the [`background`] in
/// color and a shadow underneath it.
pub mod header_background {
    use super::*;

    ensogl_core::define_shape_system! {
        above = [background, list_view::background];
        (style:Style, color:Vector4, height: f32, shadow_height_multiplier: f32) {
            let color = Var::<color::Rgba>::from(color);
            let width: Var<Pixels> = "input_size.x".into();
            let height: Var<Pixels> = height.into();
            let bg = Rect((width.clone(), height.clone())).fill(color);
            // We use wider and shorter rect for the shadow because of the visual artifacts that
            // will appear otherwise:
            // 1. Rounded corners of the shadow are visible if the rect is too narrow. By widening
            //    it we keep the shadow sharp and flat for the whole width of the header.
            // 2. Visual glitching similar to z-fighting occurs on the border of the elements
            //    when the shadow rect has the exact same size as the background. We shrink the
            //    height by 1 pixel to avoid it.
            let shadow_rect = Rect((width * 2.0, height - 1.0.px()));
            let mut shadow_parameters = shadow::parameters_from_style_path(style, theme::header::shadow);
            shadow_parameters.size = shadow_parameters.size * shadow_height_multiplier;
            let shadow = shadow::from_shape_with_parameters(shadow_rect.into(), shadow_parameters);
            (shadow + bg).into()
        }
    }
}


// === Header Overlay ===

/// The transparent overlay over Component Group View Header, used for capturing mouse events.
pub mod header_overlay {
    use super::*;

    use ensogl_core::display::shape::constants::HOVER_COLOR;

    ensogl_core::define_shape_system! {
        above = [background];
        () {
            let bg_color = HOVER_COLOR;
            Plane().fill(bg_color).into()
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
    shadow_size:    f32,
}

impl HeaderGeometry {
    fn from_style(style: &StyleWatchFrp, network: &frp::Network) -> frp::Sampler<Self> {
        let height = style.get_number(theme::header::height);
        let padding_left = style.get_number(theme::header::padding::left);
        let padding_right = style.get_number(theme::header::padding::right);
        let padding_bottom = style.get_number(theme::header::padding::bottom);
        let shadow_size = style.get_number(theme::header::shadow::size);

        frp::extend! { network
            init <- source_();
            theme <- all_with6(&init,&height,&padding_left,&padding_right,&padding_bottom,&shadow_size,
                |_,&height,&padding_left,&padding_right,&padding_bottom,&shadow_size|
                    Self{height,padding_left,padding_right,padding_bottom,shadow_size}
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
        /// Accept the currently selected suggestion. Should be bound to "Suggestion Acceptance Key"
        /// described in
        /// [Component Browser Design Doc](https://github.com/enso-org/design/blob/main/epics/component-browser/design.md#key-binding-dictionary)
        accept_suggestion(),
        set_header(String),
        set_entries(list_view::entry::AnyModelProvider<Entry>),
        set_color(color::Rgba),
        set_dimmed(bool),
        set_width(f32),
        /// Sets the y-position of the header from the top of the component group.
        ///
        /// It can't move past the top and bottom borders of the component group.
        ///
        /// We use it to simulate entries scrolling with a fixed-position header. Though in fact we
        /// move the header down while moving the whole component group up.
        set_header_pos(f32),
    }
    Output {
        selected_entry(Option<entry::Id>),
        suggestion_accepted(entry::Id),
        expression_accepted(entry::Id),
        is_header_selected(bool),
        header_accepted(),
        selection_size(Vector2<f32>),
        selection_position_target(Vector2<f32>),
        size(Vector2<f32>)
    }
}

impl component::Frp<Model> for Frp {
    fn init(api: &Self::Private, app: &Application, model: &Model, style: &StyleWatchFrp) {
        let network = &api.network;
        let mouse_position = app.display.default_scene.mouse.frp.position.clone_ref();
        let input = &api.input;
        let out = &api.output;
        let header_text_font = style.get_text(theme::header::text::font);
        let header_text_size = style.get_number(theme::header::text::size);


        // === Geometry ===

        frp::extend! { network
            let header_geometry = HeaderGeometry::from_style(style, network);
            height <- all_with(&input.set_entries, &header_geometry, |entries, header_geom| {
                entries.entry_count() as f32 * list_view::entry::HEIGHT + header_geom.height
            });
            out.size <+ all_with(&input.set_width, &height, |w, h| Vector2(*w, *h));
            size_and_header_geometry <- all3(&out.size, &header_geometry, &input.set_header_pos);
            eval size_and_header_geometry(((size, hdr_geom, hdr_pos))
                model.resize(*size, *hdr_geom, *hdr_pos)
            );


            // === Show/hide shadow ===

            shadow <- input.set_header_pos.map(|p| (*p / HEADER_SHADOW_PEAK).min(1.0));
            eval shadow((v) model.header_background.shadow_height_multiplier.set(*v));
        }


        // === Colors ===

        fn mix(colors: &(color::Rgba, color::Rgba), coefficient: &f32) -> color::Rgba {
            color::mix(colors.0, colors.1, *coefficient)
        }
        let app_bg_color = style.get_color(ensogl_hardcoded_theme::application::background);
        let header_intensity = style.get_number(theme::header::text::color_intensity);
        let bg_intensity = style.get_number(theme::background_color_intensity);
        let dimmed_intensity = style.get_number(theme::dimmed_color_intensity);
        let entry_text_color = style.get_color(theme::entries::text::color);
        frp::extend! { network
            init <- source_();
            one <- init.constant(1.0);
            intensity <- input.set_dimmed.switch(&one, &dimmed_intensity);
            app_bg_color <- all(&app_bg_color, &init)._0();
            app_bg_and_input_color <- all(&app_bg_color, &input.set_color);
            main_color <- app_bg_and_input_color.all_with(&intensity, mix);
            app_bg_and_main_color <- all(&app_bg_color, &main_color);
            header_color <- app_bg_and_main_color.all_with(&header_intensity, mix);
            bg_color <- app_bg_and_main_color.all_with(&bg_intensity, mix);
            app_bg_and_entry_text_color <- all(&app_bg_color, &entry_text_color);
            entry_color_with_intensity <- app_bg_and_entry_text_color.all_with(&intensity, mix);
            entry_color_sampler <- entry_color_with_intensity.sampler();
        }
        let params = entry::Params { color: entry_color_sampler };
        model.entries.set_entry_params_and_recreate_entries(params);


        // === Header ===

        frp::extend! { network
            header_text_font <- all(&header_text_font, &init)._0();
            model.header.set_font <+ header_text_font;
            header_text_size <- all(&header_text_size, &init)._0();
            model.header.set_default_text_size <+ header_text_size.map(|v| text::Size(*v));
            _set_header <- input.set_header.map2(&size_and_header_geometry, f!(
                (text, (size, hdr_geom, _)) {
                    model.header_text.replace(text.clone());
                    model.update_header_width(*size, *hdr_geom);
                })
            );
            model.header.set_default_color <+ header_color;
            eval bg_color((c) model.background.color.set(c.into()));
            eval bg_color((c) model.header_background.color.set(c.into()));
        }


        // === Suggestion Acceptance ===

        frp::extend! { network
            accepted_entry <- model.entries.selected_entry.sample(&input.accept_suggestion);
            out.suggestion_accepted <+ accepted_entry.filter_map(|e| *e);
            header_accepted_by_frp <- input.accept_suggestion.gate(&out.is_header_selected);
            header_accepted_by_mouse <- model.header_overlay.events.mouse_down.constant(());
            header_accepted <- any(header_accepted_by_frp, header_accepted_by_mouse);
            out.header_accepted <+ header_accepted;
            out.expression_accepted <+ model.entries.chosen_entry.filter_map(|e| *e);
        }


        // === Entries ===

        frp::extend! { network
            model.entries.set_entries <+ input.set_entries;
            out.selected_entry <+ model.entries.selected_entry;
        }


        // === Selection ===

        let overlay_events = &model.header_overlay.events;
        frp::extend! { network
            let moved_out_above = model.entries.tried_to_move_out_above.clone_ref();
            is_mouse_over <- bool(&overlay_events.mouse_out, &overlay_events.mouse_over);
            mouse_moved <- mouse_position.on_change().constant(());
            mouse_moved_over_header <- mouse_moved.gate(&is_mouse_over);

            select_header <- any(moved_out_above, mouse_moved_over_header, out.header_accepted);
            deselect_header <- model.entries.selected_entry.filter_map(|entry| *entry);
            out.is_header_selected <+ bool(&deselect_header, &select_header);
            model.entries.select_entry <+ select_header.constant(None);

            out.selection_position_target <+ all_with4(
                &out.is_header_selected,
                &out.size,
                &header_geometry,
                &model.entries.selection_position_target,
                f!((h_sel, size, h, esp) model.selection_position(*h_sel, *size, *h, *esp))
            );
        }

        init.emit(());
    }

    fn default_shortcuts() -> Vec<Shortcut> {
        use ensogl_core::application::shortcut::ActionType::*;
        (&[(Press, "tab", "accept_suggestion")])
            .iter()
            .map(|(a, b, c)| View::self_shortcut(*a, *b, *c))
            .collect()
    }
}



// ==============
// === Layers ===
// ==============

/// A set of scene layers shared by every component group.
///
/// A component group consists of a several shapes with a strict rendering order. The order of the
/// fields of this struct represents the rendering order of layers, with `background` being the
/// bottom-most and `header_text` being the top-most.
#[derive(Debug, Clone, CloneRef)]
pub struct Layers {
    background:  layer::Layer,
    text:        layer::Layer,
    header:      layer::Layer,
    header_text: layer::Layer,
}

impl Layers {
    /// Constructor.
    ///
    /// A `camera` will be used to render all layers. Layers will be attached to a `parent_layer` as
    /// sublayers.
    pub fn new(logger: &Logger, camera: &Camera2d, parent_layer: &layer::Layer) -> Self {
        let background = layer::Layer::new_with_cam(logger.clone_ref(), camera);
        let text = layer::Layer::new_with_cam(logger.clone_ref(), camera);
        let header = layer::Layer::new_with_cam(logger.clone_ref(), camera);
        let header_text = layer::Layer::new_with_cam(logger.clone_ref(), camera);

        background.add_sublayer(&text);
        background.add_sublayer(&header);
        header.add_sublayer(&header_text);

        parent_layer.add_sublayer(&background);

        Self { background, header, text, header_text }
    }
}


// =============
// === Model ===
// =============

/// The Model of the [`View`] component.
#[derive(Clone, CloneRef, Debug)]
pub struct Model {
    display_object:    display::object::Instance,
    header:            text::Area,
    header_background: header_background::View,
    header_text:       Rc<RefCell<String>>,
    header_overlay:    header_overlay::View,
    background:        background::View,
    entries:           list_view::ListView<Entry>,
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
        let header_overlay = header_overlay::View::new(&logger);
        let background = background::View::new(&logger);
        let header_background = header_background::View::new(&logger);
        let header = text::Area::new(app);
        let entries = app.new_view::<list_view::ListView<Entry>>();
        entries.set_style_prefix(entry::STYLE_PATH);
        entries.set_background_color(HOVER_COLOR);
        entries.show_background_shadow(false);
        entries.set_background_corners_radius(0.0);
        entries.hide_selection();
        display_object.add_child(&background);
        display_object.add_child(&header_background);
        display_object.add_child(&header);
        display_object.add_child(&header_overlay);
        display_object.add_child(&entries);

        Model {
            display_object,
            header_overlay,
            header,
            header_text,
            background,
            header_background,
            entries,
        }
    }
}

impl Model {
    /// Assign a set of layers to render the component group in. Must be called after constructing
    /// the [`View`].
    pub fn set_layers(&self, layers: &Layers) {
        layers.background.add_exclusive(&self.background);
        layers.header_text.add_exclusive(&self.header_overlay);
        layers.background.add_exclusive(&self.entries);
        self.entries.set_label_layer(&layers.text);
        layers.header.add_exclusive(&self.header_background);
        self.header.add_to_scene_layer(&layers.header_text);
    }

    fn resize(&self, size: Vector2, header_geometry: HeaderGeometry, header_pos: f32) {
        // === Background ===

        self.background.size.set(size);


        // === Header Text ===

        let header_padding_left = header_geometry.padding_left;
        let header_text_x = -size.x / 2.0 + header_padding_left;
        let header_text_height = self.header.height.value();
        let header_padding_bottom = header_geometry.padding_bottom;
        let header_height = header_geometry.height;
        let half_header_height = header_height / 2.0;
        let header_center_y = size.y / 2.0 - half_header_height;
        let header_center_y = header_center_y - header_pos;
        let header_center_y = header_center_y.max(-size.y / 2.0 + half_header_height);
        let header_center_y = header_center_y.min(size.y / 2.0 - half_header_height);
        let header_bottom_y = header_center_y - half_header_height;
        let header_text_y = header_bottom_y + header_text_height + header_padding_bottom;
        self.header.set_position_xy(Vector2(header_text_x, header_text_y));
        self.update_header_width(size, header_geometry);


        // === Header Background ===

        self.header_background.height.set(header_height);
        let shadow_size = header_geometry.shadow_size;
        let distance_to_bottom = (-size.y / 2.0 - header_bottom_y).abs();
        // We need to render both the header background and the shadow below it, so we add
        // `shadow_size` and `header_height` to calculate the final `size` of the
        // `header_background` shape. We use `shadow_size * 2.0`, because the shadow extends by
        // `shadow_size` in each direction around the base shape, not only down. We cap the
        // `shadow_size` by the distance to the bottom of the component group so that the shadow is
        // not visible outside the component group background.
        let shadow_size = shadow_size.min(distance_to_bottom);
        let header_background_height = header_height + shadow_size * 2.0;
        self.header_background.size.set(Vector2(size.x, header_background_height));
        self.header_background.set_position_y(header_center_y);


        // === Header Overlay ===

        self.header_overlay.set_position_y(header_center_y);
        self.header_overlay.size.set(Vector2(size.x, header_height));


        // === Entries ===

        self.entries.resize(size - Vector2(0.0, header_height));
        self.entries.set_position_y(-header_height / 2.0);
    }

    fn update_header_width(&self, size: Vector2, header_geometry: HeaderGeometry) {
        let header_padding_left = header_geometry.padding_left;
        let header_padding_right = header_geometry.padding_right;
        let max_text_width = size.x - header_padding_left - header_padding_right;
        self.header.set_content_truncated(self.header_text.borrow().clone(), max_text_width);
    }

    fn selection_position(
        &self,
        is_header_selected: bool,
        size: Vector2,
        header_geometry: HeaderGeometry,
        entries_selection_position: Vector2,
    ) -> Vector2 {
        if is_header_selected {
            Vector2(0.0, size.y / 2.0 - header_geometry.height / 2.0)
        } else {
            self.entries.position().xy() + entries_selection_position
        }
    }
}



// ============
// === View ===
// ============

/// A widget for displaying the entries and name of a Component Group.
///
/// The widget is rendered as a header label, a list of entries below it, and a colored background.
/// It does not display the selection widget - because the selection jump between various Component
/// Groups, the parent (Component List Panel) should own the selection widget; the Component Group
/// View provides the information where the widget should be placed when it's focused.
///
/// To learn more about Component Groups, see the [Component Browser Design
/// Document](https://github.com/enso-org/design/blob/e6cffec2dd6d16688164f04a4ef0d9dff998c3e7/epics/component-browser/design.md).
pub type View = component::ComponentView<Model, Frp>;



// =============
// === Tests ===
// =============

#[cfg(test)]
mod tests {
    use super::*;
    use enso_frp::future::EventOutputExt;
    use ensogl_core::control::io::mouse;
    use ensogl_list_view::entry::AnyModelProvider;

    macro_rules! expect_entry_selected {
        ($cgv:ident, $id:expr$(, $argv:tt)?) => {
            assert_eq!($cgv.selected_entry.value(), Some($id)$(, $argv)?);
            assert!(!$cgv.is_header_selected.value()$(, $argv)?);
        };
    }

    macro_rules! expect_header_selected {
        ($cgv:ident$(, $argv:tt)?) => {
            assert_eq!($cgv.selected_entry.value(), None$(, $argv)?);
            assert!($cgv.is_header_selected.value()$(, $argv)?);
        };
    }

    struct Test {
        app: Application,
        cgv: View,
    }

    impl Test {
        fn set_up() -> Self {
            let app = Application::new("root");
            ensogl_hardcoded_theme::builtin::light::register(&app);
            ensogl_hardcoded_theme::builtin::light::enable(&app);
            let cgv = View::new(&app);
            cgv.set_width(100.0);
            let entries = AnyModelProvider::<Entry>::new(vec!["Entry 1", "Entry 2", "Entry 3"]);
            cgv.set_entries(entries);
            Test { app, cgv }
        }

        fn move_mouse_over_header(&self) {
            let pos_over_header = Vector2(0.0, self.cgv.size.value().y / 2.0 - 10.0);
            let mouse_move = Vector2(5.0, 0.0);
            self.cgv.model().header_overlay.events.mouse_over.emit(());
            self.app.display.default_scene.mouse.frp.position.emit(pos_over_header);
            self.app.display.default_scene.mouse.frp.position.emit(pos_over_header + mouse_move);
        }
    }

    #[test]
    fn navigating_entries_with_keyboard() {
        let Test { app: _, cgv } = Test::set_up();
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 0);
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 1);
        cgv.model().entries.move_selection_up();
        expect_entry_selected!(cgv, 0);
        cgv.model().entries.move_selection_up();
        expect_header_selected!(cgv);
        cgv.model().entries.move_selection_up();
        expect_header_selected!(cgv);
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 0);
    }

    #[test]
    fn navigating_entries_with_keyboard_after_hovering_header() {
        let test = Test::set_up();
        let cgv = &test.cgv;
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 0);
        test.move_mouse_over_header();
        expect_header_selected!(cgv);
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 0);
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 1);
        test.move_mouse_over_header();
        expect_header_selected!(cgv);
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 0);
    }

    #[test]
    fn accepting_header() {
        let Test { app: _, cgv } = Test::set_up();
        cgv.model().entries.move_selection_down();
        expect_entry_selected!(cgv, 0);
        let expected_header_accepted = cgv.header_accepted.next_event();
        cgv.model().header_overlay.events.mouse_down.emit(mouse::Button::Button0);
        expected_header_accepted.expect();
        expect_header_selected!(cgv);

        let expected_header_accepted = cgv.header_accepted.next_event();
        cgv.accept_suggestion();
        expected_header_accepted.expect();
        expect_header_selected!(cgv);

        // When header is not selected, we should not be able to accept it with keyboard
        let expected_header_accepted = cgv.header_accepted.next_event();
        cgv.model().entries.select_entry(0);
        expect_entry_selected!(cgv, 0);
        cgv.accept_suggestion();
        expected_header_accepted.expect_not();
    }
}
