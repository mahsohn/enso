// ====================
// === Define Macro ===
// ====================

/// Macro for defining icon.
///
/// The macro takes many modules with attached "variant name". Inside the modules, there should
/// be icon defined with `ensogl::define_shape_system!` macro. The macro will generate also an
/// enum called `Id` gathering all icon' "variant names". The enum will allow for dynamically
/// creating given icon shape view (returned as `Box<dyn ensogl::display::Object>`.
///
/// # Example
///
/// ```
/// use ensogl::prelude::*;
/// use ensogl::display::shape::*;
/// use ide_view_component_group::define_icons;
///
/// define_icons! {
///     /// The example of icon.
///     pub mod icon1(Icon1) {
///         // This is a normal module and you may define whatever you want. It must however
///         // define shape system with the macro below; otherwise the generated code wont compile.
///         //
///         // `use super::*` import is added silently.
///         ensogl::define_shape_system! {
///             (style:Style) {
///                 Plane().into()
///             }
///         }
///     }
///
///     pub mod icon2(Icon2) {
///         ensogl::define_shape_system! {
///             (style:Style, color:Vector4) {
///                 Plane().fill(color).into()
///             }
///         }
///     }
/// }
///
/// fn main () {
///     let app = ensogl::application::Application::new("root");
///     let logger = Logger::new("icon");
///     let icon1 = Id::Icon1.create_shape(&logger, Vector2(10.0, 10.0));///
///     let icon2_id: Id = "Icon2".parse().unwrap();
///     assert_eq!(icon2_id, Id::Icon2);
///     let icon2 = icon2_id.create_shape(&logger, Vector2(11.0, 11.0));
///     app.display.default_scene.add_child(&*icon1);
///     app.display.default_scene.add_child(&*icon2);
///
///     // Invalid icon
///     let icon3 = "Icon3".parse();
///     assert_eq!(icon3, Err(ide_view_component_group::icon::UnknownIcon));
/// }
#[macro_export]
macro_rules! define_icons {
    ($($(#[doc=$docs:tt])* pub mod $name:ident($variant:ident) { $($content:tt)* })*) => {
        $(
        $(#[doc=$docs])*
        pub mod $name {
            use super::*;

            $($content)*
        }
        )*

        #[derive(Copy, Clone, Debug, Eq, PartialEq)]
        pub enum Id {
            $($variant,)*
        }

        impl Id {
            pub fn create_shape(&self, logger: impl AnyLogger, size: Vector2) -> Box<dyn $crate::ensogl::display::Object> {
                match self {
                    $(
                    Self::$variant => {
                        let view = $name::View::new(logger);
                        view.size.set(size);
                        Box::new(view)
                    }
                    )*
                }
            }
        }

        impl std::str::FromStr for Id {
            type Err = $crate::icon::UnknownIcon;
            fn from_str(s: &str) -> Result<Id, Self::Err> {
                match s {
                    $(stringify!($variant) => Ok(Self::$variant),)*
                    name => Err(Self::Err {name: name.to_owned() }),
                }
            }
        }
    }
}
