//! Launch check: lets the window draw, saves a screenshot and closes it.

use egui::{ColorImage, Context, ViewportCommand};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

const WARM_UP_FRAMES: u32 = 30;

/// The screenshot outcome, shared with `main` once the window has closed.
pub type Outcome = Arc<Mutex<Option<io::Result<()>>>>;

/// Saves one screenshot of the window after a few frames, then closes it.
pub struct Capture {
    path: PathBuf,
    frames: u32,
    requested: bool,
    outcome: Outcome,
}

impl Capture {
    /// Will save the window to `path` and record the result in `outcome`.
    #[must_use]
    pub const fn new(path: PathBuf, outcome: Outcome) -> Self {
        Self {
            path,
            frames: 0,
            requested: false,
            outcome,
        }
    }

    /// Called once per frame. Requests the screenshot after the warm-up.
    pub fn step(&mut self, ctx: &Context) {
        if self.requested {
            return;
        }
        self.frames += 1;
        ctx.request_repaint();
        if self.frames < WARM_UP_FRAMES {
            return;
        }
        self.requested = true;
        let path = self.path.clone();
        let outcome = Arc::clone(&self.outcome);
        let close = ctx.clone();
        ctx.request_screenshot(move |image| {
            let saved = save_png(&image, &path);
            *outcome.lock().unwrap_or_else(PoisonError::into_inner) = Some(saved);
            close.send_viewport_cmd(ViewportCommand::Close);
        });
    }
}

fn save_png(image: &ColorImage, path: &Path) -> io::Result<()> {
    let [width, height] = image.size;
    let (Ok(width), Ok(height)) = (u32::try_from(width), u32::try_from(height)) else {
        return Err(io::Error::other("screenshot is too large"));
    };
    let pixels = image.as_raw().to_vec();
    let buffer = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| io::Error::other("screenshot size does not match its pixels"))?;
    buffer.save(path).map_err(io::Error::other)
}
