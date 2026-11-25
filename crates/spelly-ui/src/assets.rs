use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// Pixelify Sans (SIL OFL 1.1, see assets/fonts/OFL-PixelifySans.txt): our display face.
pub const PIXEL_FONT: &[u8] = include_bytes!("../assets/fonts/PixelifySans.ttf");
pub const PIXEL: &str = "Pixelify Sans";

/// Everything the window draws, baked into the binary.
pub struct Assets;

const FILES: &[(&str, &[u8])] = &[
    ("mascot/kitten.png", include_bytes!("../../../assets/mascot/kitten.png")),
    ("mascot/sleepy.png", include_bytes!("../../../assets/mascot/sleepy.png")),
    ("mascot/yawn.png", include_bytes!("../../../assets/mascot/yawn.png")),
    ("mascot/tuxedo.png", include_bytes!("../../../assets/mascot/tuxedo.png")),
    ("icons/home.svg", include_bytes!("../assets/icons/home.svg")),
    ("icons/clock.svg", include_bytes!("../assets/icons/clock.svg")),
    ("icons/book.svg", include_bytes!("../assets/icons/book.svg")),
    ("icons/sliders.svg", include_bytes!("../assets/icons/sliders.svg")),
    ("icons/x.svg", include_bytes!("../assets/icons/x.svg")),
    ("icons/plus.svg", include_bytes!("../assets/icons/plus.svg")),
    ("icons/check.svg", include_bytes!("../assets/icons/check.svg")),
    ("icons/search.svg", include_bytes!("../assets/icons/search.svg")),
    ("icons/folder.svg", include_bytes!("../assets/icons/folder.svg")),
    ("icons/arrow-right.svg", include_bytes!("../assets/icons/arrow-right.svg")),
    ("icons/undo.svg", include_bytes!("../assets/icons/undo.svg")),
    ("icons/alert.svg", include_bytes!("../assets/icons/alert.svg")),
    ("icons/chevron-right.svg", include_bytes!("../assets/icons/chevron-right.svg")),
    ("icons/chevron-down.svg", include_bytes!("../assets/icons/chevron-down.svg")),
];

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(FILES.iter().find(|(p, _)| *p == path).map(|(_, b)| Cow::Borrowed(*b)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(FILES.iter().filter(|(p, _)| p.starts_with(path)).map(|(p, _)| SharedString::from(*p)).collect())
    }
}
