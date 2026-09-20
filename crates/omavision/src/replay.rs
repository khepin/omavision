//! OMAVISION_REPLAY="Right Right w i c k Down Return" feeds keys through the real input
//! pipeline after startup, so agents can drive the app without OS-level keystroke rights.
use crate::MainWindow;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, SharedString};

pub fn keys(ui: &MainWindow, script: String) -> slint::Timer {
    let keys: Vec<SharedString> = script
        .split_whitespace()
        .map(|k| match k {
            "Up" => Key::UpArrow.into(),
            "Down" => Key::DownArrow.into(),
            "Left" => Key::LeftArrow.into(),
            "Right" => Key::RightArrow.into(),
            "Return" | "Enter" => Key::Return.into(),
            "Escape" | "Esc" => Key::Escape.into(),
            "Backspace" => Key::Backspace.into(),
            "PageUp" => Key::PageUp.into(),
            "PageDown" => Key::PageDown.into(),
            "Home" => Key::Home.into(),
            "End" => Key::End.into(),
            "Space" => " ".into(),
            other => other.into(),
        })
        .collect();
    let weak = ui.as_weak();
    let mut i = 0;
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(400), move || {
        let Some(ui) = weak.upgrade() else { return };
        if i >= keys.len() {
            return;
        }
        let text = keys[i].clone();
        i += 1;
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text });
    });
    timer
}
