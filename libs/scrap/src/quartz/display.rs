use std::mem;

use super::ffi::*;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(C)]
pub struct Display(u32);

fn native_pixel_size(display: u32) -> Option<(usize, usize)> {
    unsafe {
        let modes = CGDisplayCopyAllDisplayModes(display, std::ptr::null_mut());
        if modes.is_null() {
            return None;
        }
        let mut res = None;
        for i in 0..CFArrayGetCount(modes) {
            let mode = CFArrayGetValueAtIndex(modes, i) as CGDisplayModeRef;
            if mode.is_null() || CGDisplayModeGetIOFlags(mode) & DISPLAY_MODE_NATIVE_FLAG == 0 {
                continue;
            }
            let (w, h) = (
                CGDisplayModeGetPixelWidth(mode),
                CGDisplayModeGetPixelHeight(mode),
            );
            if w > 0 && h > 0 {
                res = Some((w, h));
                break;
            }
        }
        CFRelease(modes);
        res
    }
}

impl Display {
    pub fn primary() -> Display {
        Display(unsafe { CGMainDisplayID() })
    }

    pub fn online() -> Result<Vec<Display>, CGError> {
        unsafe {
            #[allow(invalid_value)]
            let mut arr: [u32; 16] = mem::MaybeUninit::uninit().assume_init();
            let mut len: u32 = 0;

            match CGGetOnlineDisplayList(16, arr.as_mut_ptr(), &mut len) {
                CGError::Success => (),
                x => return Err(x),
            }

            let mut res = Vec::with_capacity(16);
            for i in 0..len as usize {
                res.push(Display(*arr.get_unchecked(i)));
            }
            Ok(res)
        }
    }

    pub fn id(self) -> u32 {
        self.0
    }

    pub fn width(self) -> usize {
        self.pixel_size().0
    }

    pub fn height(self) -> usize {
        self.pixel_size().1
    }

    // On Retina displays macOS may render into a framebuffer larger than the panel when the
    // user picks a scaled mode, so cap the size at the panel's own pixel size: the extra
    // pixels cost CPU and bandwidth while showing nothing the user can see.
    fn pixel_size(self) -> (usize, usize) {
        let w = unsafe { CGDisplayPixelsWide(self.0) };
        let h = unsafe { CGDisplayPixelsHigh(self.0) };
        let s = if *super::ENABLE_RETINA.lock().unwrap() {
            unsafe { BackingScaleFactor(self.0) as f64 }
        } else {
            1.
        };
        let (w, h) = if s > 1. {
            (
                ((w as f64) * s).round() as usize,
                ((h as f64) * s).round() as usize,
            )
        } else {
            (w, h)
        };
        match native_pixel_size(self.0) {
            Some((pw, ph)) => (w.min(pw), h.min(ph)),
            None => (w, h),
        }
    }

    pub fn is_builtin(self) -> bool {
        unsafe { CGDisplayIsBuiltin(self.0) != 0 }
    }

    pub fn is_primary(self) -> bool {
        unsafe { CGDisplayIsMain(self.0) != 0 }
    }

    pub fn is_active(self) -> bool {
        unsafe { CGDisplayIsActive(self.0) != 0 }
    }

    pub fn is_online(self) -> bool {
        unsafe { CGDisplayIsOnline(self.0) != 0 }
    }

    // Derived from `width` so that callers can convert between the pixel size we capture
    // and the logical size that mouse events are expressed in.
    pub fn scale(self) -> f64 {
        let w = unsafe { CGDisplayPixelsWide(self.0) };
        if w == 0 {
            return 1.;
        }
        self.width() as f64 / w as f64
    }

    pub fn bounds(self) -> CGRect {
        unsafe { CGDisplayBounds(self.0) }
    }
}
