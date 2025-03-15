use serde::Deserialize;
use wayrs_protocols::wlr_layer_shell_unstable_v1::zwlr_layer_surface_v1::Anchor;

/// Light wrapper around `Anchor` which also supports the "no anchor" value.
///
/// This type is also requires to derive `Deserialize` for the foreign type.
#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum ConfigAnchor {
    #[default]
    Center,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Convert this anchor into the type expected by `wayrs`.
impl From<ConfigAnchor> for Anchor {
    fn from(value: ConfigAnchor) -> Self {
        match value {
            ConfigAnchor::Center => Anchor::empty(),
            ConfigAnchor::Top => Anchor::Top,
            ConfigAnchor::Bottom => Anchor::Bottom,
            ConfigAnchor::Left => Anchor::Left,
            ConfigAnchor::Right => Anchor::Right,
            ConfigAnchor::TopLeft => Anchor::Top | Anchor::Left,
            ConfigAnchor::TopRight => Anchor::Top | Anchor::Right,
            ConfigAnchor::BottomLeft => Anchor::Bottom | Anchor::Left,
            ConfigAnchor::BottomRight => Anchor::Bottom | Anchor::Right,
        }
    }
}
