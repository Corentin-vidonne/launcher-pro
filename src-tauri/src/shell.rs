//! Appels Windows : lancement (ShellExecute) et extraction d'icônes en PNG.

use std::ffi::c_void;
use std::path::Path;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, ShellExecuteW, SIIGBF_BIGGERSIZEOK,
    SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::model::Launch;

/// Ouvre une cible comme le ferait un double-clic : exécutable, protocole (`steam://…`), dossier.
/// ShellExecute gère aussi l'élévation UAC des jeux qui l'exigent.
pub fn open(target: &str, args: Option<&str>, cwd: Option<&str>) -> Result<(), String> {
    let target_w = HSTRING::from(target);
    let args_w = args.map(HSTRING::from);
    let cwd_w = cwd.map(HSTRING::from);
    let as_ptr = |s: &Option<HSTRING>| s.as_ref().map(|h| PCWSTR(h.as_ptr())).unwrap_or(PCWSTR::null());
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &target_w,
            as_ptr(&args_w),
            as_ptr(&cwd_w),
            SW_SHOWNORMAL,
        )
    };
    // Convention ShellExecute : une valeur > 32 signale un succès.
    if result.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("Impossible d'ouvrir « {target} » (code {})", result.0 as isize))
    }
}

pub fn launch(launch: &Launch) -> Result<(), String> {
    match launch {
        Launch::Uri { uri } => open(uri, None, None),
        Launch::Exe { path, args, cwd } => {
            let parent = Path::new(path).parent().map(|p| p.to_string_lossy().to_string());
            open(path, args.as_deref(), cwd.as_deref().or(parent.as_deref()))
        }
        Launch::None => Err("Aucune commande de lancement connue pour cet élément.".into()),
    }
}

/// Icône d'un fichier (exe, raccourci…) rendue par le shell, encodée en PNG.
pub fn icon_png(path: &Path, size: i32) -> Option<Vec<u8>> {
    unsafe {
        // Sans effet si COM est déjà initialisé sur ce thread.
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None).ok()?;
        let hbitmap = factory
            .GetImage(SIZE { cx: size, cy: size }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK)
            .ok()?;
        let object = HGDIOBJ(hbitmap.0);

        let mut bitmap = BITMAP::default();
        let read = GetObjectW(
            object,
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bitmap as *mut BITMAP as *mut c_void),
        );
        let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
        if read == 0 || width <= 0 || height <= 0 || width > 1024 || height > 1024 {
            let _ = DeleteObject(object);
            return None;
        }

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // lignes de haut en bas
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            dc,
            hbitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        let _ = DeleteObject(object);
        if lines == 0 {
            return None;
        }

        // BGRA prémultiplié → RGBA droit.
        let mut has_alpha = false;
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
            let a = px[3] as u32;
            if a != 0 {
                has_alpha = true;
                if a < 255 {
                    for c in &mut px[..3] {
                        *c = ((*c as u32 * 255) / a).min(255) as u8;
                    }
                }
            }
        }
        if !has_alpha {
            // Vieilles icônes sans canal alpha : tout est opaque.
            for px in pixels.chunks_exact_mut(4) {
                px[3] = 255;
            }
        }

        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, width as u32, height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().ok()?;
            writer.write_image_data(&pixels).ok()?;
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test opens_process -- --ignored`
    #[test]
    #[ignore]
    fn opens_process() {
        assert!(open("cmd.exe", Some("/c exit"), None).is_ok());
        assert!(open("Z:\\introuvable\\jeu.exe", None, None).is_err());
        assert!(launch(&Launch::None).is_err());
    }
}
