//! Звуки (FR-SND-1): встроенные CC0-файлы или свой файл
//! (aiff/wav/mp3/m4a); громкость 0.0–1.0. Воспроизведение через NSSound.

use objc2::AnyThread;
use objc2_app_kit::NSSound;
use objc2_foundation::{NSData, NSString, NSURL};

/// Встроенные CC0-файлы (короткие бипы, сгенерированы проектом).
pub fn builtin_kind(kind: &str) -> &'static [u8] {
    match kind {
        "switch" => include_bytes!("../../../../resources/sounds/switch.wav"),
        _ => include_bytes!("../../../../resources/sounds/fix.wav"),
    }
}

/// Воспроизвести Звук вида `kind` (`switch` / `fix`).
/// `custom` — путь пользовательского файла; `volume` 0.0–1.0.
pub fn play(kind: &str, custom: Option<&str>, volume: f32) {
    if let Some(path) = custom.filter(|p| !p.is_empty()) {
        play_path(path, volume);
        return;
    }
    play_bytes(builtin_kind(kind), volume);
}

fn play_bytes(data: &[u8], volume: f32) {
    let sound = unsafe {
        // SAFETY: конструктор Cocoa; данные живут до конца play().
        NSSound::initWithData(
            <NSSound as objc2::AnyThread>::alloc(),
            &NSData::with_bytes(data),
        )
    };
    if let Some(s) = sound {
        s.setVolume(volume.clamp(0.0, 1.0));
        s.play();
    } else {
        tracing::warn!("звук: NSSound не создался из встроенных данных");
    }
}

fn play_path(path: &str, volume: f32) {
    let url = NSURL::fileURLWithPath(&NSString::from_str(path));
    let sound = unsafe {
        // SAFETY: конструктор Cocoa; по ссылке достаточно файла.
        NSSound::initWithContentsOfURL_byReference(NSSound::alloc(), &url, false)
    };
    if let Some(s) = sound {
        s.setVolume(volume.clamp(0.0, 1.0));
        s.play();
    } else {
        tracing::warn!("звук: файл не открылся: {path}");
    }
}
