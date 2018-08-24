#![windows_subsystem = "windows"]

extern crate iui;
extern crate romhack_backend;

use iui::controls::{Button, HorizontalSeparator, Label, Spacer, VerticalBox};
use iui::prelude::*;
use romhack_backend::{apply_patch, DontPrint};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

struct State {
    iso: Option<PathBuf>,
    patch: Option<PathBuf>,
    ui: UI,
    apply: Button,
}

impl State {
    fn update(&mut self) {
        if self.iso.is_some() && self.patch.is_some() {
            self.ui.set_enabled(self.apply.clone(), true);
        }
    }
}

fn main() {
    let mut ui = UI::init().unwrap();
    let mut window = Window::new(&ui, "GameCube ISO Patcher", 300, 100, WindowType::NoMenubar);

    let mut vbox = VerticalBox::new(&ui);
    vbox.set_padded(&ui, true);

    let mut apply_button = Button::new(&ui, "Apply");
    ui.set_enabled(apply_button.clone(), false);

    let state = Rc::new(RefCell::new(State {
        iso: None,
        patch: None,
        ui: ui.clone(),
        apply: apply_button.clone(),
    }));

    let patch_label = Label::new(&ui, "No Patch selected");
    let mut patch_button = Button::new(&ui, "Open Patch");
    patch_button.on_clicked(&ui, {
        let ui = ui.clone();
        let mut label = patch_label.clone();
        let window = window.clone();
        let state = state.clone();
        move |_btn| {
            if let Some(path) = window.open_file(&ui) {
                label.set_text(
                    &ui,
                    &path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default(),
                );
                let mut state = state.borrow_mut();
                state.patch = Some(path);
                state.update();
            }
        }
    });

    let iso_label = Label::new(&ui, "No ISO selected");
    let mut iso_button = Button::new(&ui, "Open ISO");
    iso_button.on_clicked(&ui, {
        let ui = ui.clone();
        let mut label = iso_label.clone();
        let window = window.clone();
        let state = state.clone();
        move |_btn| {
            if let Some(path) = window.open_file(&ui) {
                label.set_text(
                    &ui,
                    &path
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default(),
                );
                let mut state = state.borrow_mut();
                state.iso = Some(path);
                state.update();
            }
        }
    });

    apply_button.on_clicked(&ui, {
        let ui = ui.clone();
        let window = window.clone();
        let state = state.clone();
        move |_btn| {
            let state = state.borrow();
            if let (Some(patch), Some(iso)) = (&state.patch, &state.iso) {
                if let Some(output) = window.save_file(&ui) {
                    apply_patch(&DontPrint, patch.to_owned(), iso.to_owned(), output).unwrap();
                }
            }
        }
    });

    vbox.append(&ui, patch_label, LayoutStrategy::Compact);
    vbox.append(&ui, patch_button, LayoutStrategy::Compact);
    vbox.append(&ui, iso_label, LayoutStrategy::Compact);
    vbox.append(&ui, iso_button, LayoutStrategy::Compact);
    vbox.append(&ui, Spacer::new(&ui), LayoutStrategy::Compact);
    vbox.append(&ui, HorizontalSeparator::new(&ui), LayoutStrategy::Compact);
    vbox.append(&ui, Spacer::new(&ui), LayoutStrategy::Compact);
    vbox.append(&ui, apply_button, LayoutStrategy::Compact);

    window.set_child(&ui, vbox);
    window.show(&ui);
    ui.main();
}
