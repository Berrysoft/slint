// Copyright © SixtyFPS GmbH <info@slint-ui.com>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-commercial

//! Helper for wasm that adds a hidden `<input>`  and process its events
//!
//! Without it, the key event are sent to the canvas and processed by winit.
//! But this winit handling doesn't show the keyboard on mobile devices, and
//! also has bugs as the modifiers are not reported the same way and we don't
//! record them.
//!
//! This just interpret the keyup and keydown events. But this is not working
//! on mobile either as we only get these for a bunch of non-printable key
//! that do not interact with the composing input. For anything else we
//! check that we get input event when no normal key are pressed, and we send
//! that as text.
//! Since the slint core lib doesn't support composition yet, when we get
//! composition event, we just send that as key, and if the composition changes,
//! we just simulate a few backspaces.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use i_slint_core::input::{KeyEventType, KeyInputEvent};
use i_slint_core::platform::WindowEvent;
use i_slint_core::window::{WindowAdapter, WindowInner};
use i_slint_core::SharedString;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::JsCast;

pub struct WasmInputHelper {
    input: web_sys::HtmlInputElement,
    canvas: web_sys::HtmlCanvasElement,
}

#[derive(Default)]
struct WasmInputState {
    /// If there was a "keydown" event received that is not part of a composition
    has_key_down: bool,
}

impl WasmInputHelper {
    #[allow(unused)]
    pub fn new(
        window_adapter: Weak<dyn WindowAdapter>,
        canvas: web_sys::HtmlCanvasElement,
    ) -> Self {
        let window = web_sys::window().unwrap();
        let input = window
            .document()
            .unwrap()
            .create_element("input")
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        let style = input.style();
        style.set_property("z-index", "-1").unwrap();
        style.set_property("position", "absolute").unwrap();
        style.set_property("left", &format!("{}px", canvas.offset_left())).unwrap();
        style.set_property("top", &format!("{}px", canvas.offset_top())).unwrap();
        style.set_property("width", &format!("{}px", canvas.offset_width())).unwrap();
        style.set_property("height", &format!("{}px", canvas.offset_height())).unwrap();
        style.set_property("opacity", "0").unwrap(); // Hide the cursor on mobile Safari
        input.set_attribute("autocapitalize", "none").unwrap(); // Otherwise everything would be capitalized as we need to clear the input
        canvas.before_with_node_1(&input).unwrap();
        let mut h = Self { input, canvas: canvas.clone() };

        let shared_state = Rc::new(RefCell::new(WasmInputState::default()));
        #[cfg(web_sys_unstable_apis)]
        if let Some(clipboard) = window.navigator().clipboard() {
            let win = window_adapter.clone();
            let clip = clipboard.clone();
            let closure_copy = Closure::wrap(Box::new(move |e: web_sys::ClipboardEvent| {
                if let Some(window_adapter) = win.upgrade() {
                    e.prevent_default();
                    let mut item = WindowInner::from_pub(&window_adapter.window())
                        .focus_item
                        .borrow()
                        .clone()
                        .upgrade();
                    if let Some(focus_item) = item.clone() {
                        if let Some(text_input) =
                            focus_item.downcast::<i_slint_core::items::TextInput>()
                        {
                            let (anchor, cursor) =
                                text_input.as_pin_ref().selection_anchor_and_cursor();
                            if anchor == cursor {
                                return;
                            }
                            let text = text_input.as_pin_ref().text();
                            let selected_text = &text[anchor..cursor];
                            clip.write_text(selected_text);
                        }
                    }
                }
            }) as Box<dyn Fn(_)>);
            window.add_event_listener_with_callback("copy", closure_copy.as_ref().unchecked_ref());
            closure_copy.forget();

            let win = window_adapter.clone();
            let closure_paste = Closure::wrap(Box::new(move |e: web_sys::ClipboardEvent| {
                if let Some(window_adapter) = win.upgrade() {
                    e.prevent_default();
                    let mut item = WindowInner::from_pub(&window_adapter.window())
                        .focus_item
                        .borrow()
                        .clone()
                        .upgrade();
                    if let Some(focus_item) = item.clone() {
                        if let Some(text_input) =
                            focus_item.downcast::<i_slint_core::items::TextInput>()
                        {
                            let copy =
                                Closure::wrap(Box::new(move |result: wasm_bindgen::JsValue| {
                                    let focus_item_clone = focus_item.clone();
                                    text_input.as_pin_ref().insert(
                                        &result.as_string().unwrap(),
                                        &window_adapter,
                                        &focus_item_clone,
                                    );
                                })
                                    as Box<dyn FnMut(wasm_bindgen::JsValue)>);
                            clipboard.read_text().then(&copy);
                            copy.forget();
                        }
                    }
                }
            }) as Box<dyn Fn(_)>);
            window
                .add_event_listener_with_callback("paste", closure_paste.as_ref().unchecked_ref());
            closure_paste.forget();
        }

        let win = window_adapter.clone();
        h.add_event_listener("blur", move |_: web_sys::Event| {
            // Make sure that the window gets marked as unfocused when the focus leaves the input
            if let Some(window_adapter) = win.upgrade() {
                let window_inner = WindowInner::from_pub(window_adapter.window());
                if !canvas.matches(":focus").unwrap_or(false) {
                    window_inner.set_active(false);
                    window_inner.set_focus(false);
                }
            }
        });
        let win = window_adapter.clone();
        let shared_state2 = shared_state.clone();
        h.add_event_listener("keydown", move |e: web_sys::KeyboardEvent| {
            if let (Some(window_adapter), Some(text)) = (win.upgrade(), event_text(&e)) {
                e.prevent_default();
                shared_state2.borrow_mut().has_key_down = true;
                window_adapter.window().dispatch_event(WindowEvent::KeyPressed { text });
            }
        });

        let win = window_adapter.clone();
        let shared_state2 = shared_state.clone();
        h.add_event_listener("keyup", move |e: web_sys::KeyboardEvent| {
            if let (Some(window_adapter), Some(text)) = (win.upgrade(), event_text(&e)) {
                e.prevent_default();
                shared_state2.borrow_mut().has_key_down = false;
                window_adapter.window().dispatch_event(WindowEvent::KeyReleased { text });
            }
        });

        let win = window_adapter.clone();
        let shared_state2 = shared_state.clone();
        let input = h.input.clone();
        h.add_event_listener("input", move |e: web_sys::InputEvent| {
            if let (Some(window_adapter), Some(data)) = (win.upgrade(), e.data()) {
                if !e.is_composing() && e.input_type() != "insertCompositionText" {
                    if !shared_state2.borrow_mut().has_key_down {
                        let text: SharedString = data.into();
                        window_adapter
                            .window()
                            .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
                        window_adapter.window().dispatch_event(WindowEvent::KeyReleased { text });
                        shared_state2.borrow_mut().has_key_down = false;
                    }
                    input.set_value("");
                }
            }
        });

        let win = window_adapter.clone();
        let input = h.input.clone();
        h.add_event_listener("compositionend", move |e: web_sys::CompositionEvent| {
            if let (Some(window_adapter), Some(data)) = (win.upgrade(), e.data()) {
                let window_inner = WindowInner::from_pub(window_adapter.window());
                window_inner.process_key_input(KeyInputEvent {
                    text: data.into(),
                    event_type: KeyEventType::CommitComposition,
                    ..Default::default()
                });
                input.set_value("");
            }
        });

        let win = window_adapter.clone();
        h.add_event_listener("compositionupdate", move |e: web_sys::CompositionEvent| {
            if let (Some(window_adapter), Some(data)) = (win.upgrade(), e.data()) {
                let window_inner = WindowInner::from_pub(window_adapter.window());
                let text: SharedString = data.into();
                let preedit_cursor_pos = text.len();
                window_inner.process_key_input(KeyInputEvent {
                    text,
                    event_type: KeyEventType::UpdateComposition,
                    preedit_selection_start: preedit_cursor_pos,
                    preedit_selection_end: preedit_cursor_pos,
                    ..Default::default()
                });
            }
        });

        h
    }

    /// Returns wether the fake input element has focus
    pub fn has_focus(&self) -> bool {
        self.input.matches(":focus").unwrap_or(false)
    }

    pub fn show(&self) {
        self.input.style().set_property("visibility", "visible").unwrap();
        self.input.focus().unwrap();
    }

    pub fn hide(&self) {
        if self.has_focus() {
            self.canvas.focus().unwrap()
        }
        self.input.style().set_property("visibility", "hidden").unwrap();
    }

    fn add_event_listener<Arg: FromWasmAbi + 'static>(
        &mut self,
        event: &str,
        closure: impl Fn(Arg) + 'static,
    ) {
        let closure = move |arg: Arg| {
            closure(arg);
            crate::event_loop::GLOBAL_PROXY.with(|global_proxy| {
                if let Ok(mut x) = global_proxy.try_borrow_mut() {
                    if let Some(proxy) = &mut *x {
                        let _ = proxy.send_event(crate::SlintUserEvent::CustomEvent {
                            event: crate::event_loop::CustomEvent::WakeEventLoopWorkaround,
                        });
                    }
                }
            });
        };
        let closure = Closure::wrap(Box::new(closure) as Box<dyn Fn(_)>);
        self.input
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

fn event_text(e: &web_sys::KeyboardEvent) -> Option<SharedString> {
    if e.is_composing() {
        return None;
    }

    let key = e.key();

    use i_slint_core::platform::Key;

    macro_rules! check_non_printable_code {
        ($($char:literal # $name:ident # $($_qt:ident)|* # $($_winit:ident)|* ;)*) => {
            match key.as_str() {
                "Tab" if e.shift_key() => return Some(Key::Backtab.into()),
                $(stringify!($name) => {
                    return Some($char.into());
                })*
                // Why did we diverge from DOM there?
                "ArrowLeft" => return Some(Key::LeftArrow.into()),
                "ArrowUp" => return Some(Key::UpArrow.into()),
                "ArrowRight" => return Some(Key::RightArrow.into()),
                "ArrowDown" => return Some(Key::DownArrow.into()),
                "Enter" => return Some(Key::Return.into()),
                _ => (),
            }
        };
    }
    i_slint_common::for_each_special_keys!(check_non_printable_code);

    let mut chars = key.chars();
    match chars.next() {
        Some(first_char) if chars.next().is_none() => Some(first_char.into()),
        _ => None,
    }
}
