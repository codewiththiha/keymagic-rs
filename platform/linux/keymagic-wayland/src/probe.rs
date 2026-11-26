//! Binds input-method-v2, grabs the keyboard, and sends engine output.

use crate::engine::{Composer, Step};
use crate::keys::{self, Keymap};
use crate::tracker::{Surrounding, Tracker};
use std::collections::HashSet;
use std::fs::File;
use std::os::fd::{AsFd, OwnedFd};
use std::process::ExitCode;
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_keyboard,
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_keyboard_grab_v2::{self, ZwpInputMethodKeyboardGrabV2},
    zwp_input_method_manager_v2::{self, ZwpInputMethodManagerV2},
    zwp_input_method_v2::{self, ZwpInputMethodV2},
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::{self, ZwpVirtualKeyboardManagerV1},
    zwp_virtual_keyboard_v1::{self, ZwpVirtualKeyboardV1},
};
use xkbcommon::xkb;

/// Protocol state for one probe run.
struct Probe {
    tracker: Tracker,
    composer: Composer,
    input_method: ZwpInputMethodV2,
    keyboard: ZwpVirtualKeyboardV1,
    grab: Option<ZwpInputMethodKeyboardGrabV2>,
    keymap: Option<Keymap>,
    keymap_file: File,
    forwarded: HashSet<u32>,
    unavailable: bool,
}

impl Dispatch<WlRegistry, GlobalListContents> for Probe {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for Probe {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpInputMethodManagerV2, ()> for Probe {
    fn event(
        _: &mut Self,
        _: &ZwpInputMethodManagerV2,
        _: zwp_input_method_manager_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardManagerV1, ()> for Probe {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardManagerV1,
        _: zwp_virtual_keyboard_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpVirtualKeyboardV1, ()> for Probe {
    fn event(
        _: &mut Self,
        _: &ZwpVirtualKeyboardV1,
        _: zwp_virtual_keyboard_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpInputMethodV2, ()> for Probe {
    fn event(
        state: &mut Self,
        _: &ZwpInputMethodV2,
        event: zwp_input_method_v2::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_input_method_v2::Event::Activate => state.tracker.activate(),
            zwp_input_method_v2::Event::Deactivate => state.tracker.deactivate(),
            zwp_input_method_v2::Event::SurroundingText {
                text,
                cursor,
                anchor,
            } => state.tracker.surrounding(Surrounding {
                text,
                cursor,
                anchor,
            }),
            zwp_input_method_v2::Event::Done => state.done(qh),
            zwp_input_method_v2::Event::Unavailable => state.unavailable = true,
            _ => {}
        }
    }
}

impl Dispatch<ZwpInputMethodKeyboardGrabV2, ()> for Probe {
    fn event(
        state: &mut Self,
        _: &ZwpInputMethodKeyboardGrabV2,
        event: zwp_input_method_keyboard_grab_v2::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_input_method_keyboard_grab_v2::Event::Keymap { fd, size, .. } => {
                state.load_keymap(fd, size);
            }
            zwp_input_method_keyboard_grab_v2::Event::Key {
                time,
                key,
                state: key_state,
                ..
            } => {
                let pressed = matches!(key_state, WEnum::Value(wl_keyboard::KeyState::Pressed));
                state.key(time, key, pressed);
            }
            zwp_input_method_keyboard_grab_v2::Event::Modifiers {
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => state.set_modifiers(mods_depressed, mods_latched, mods_locked, group),
            _ => {}
        }
    }
}

impl Probe {
    fn done(&mut self, qh: &QueueHandle<Self>) {
        let was_active = self.tracker.is_active();
        self.tracker.done();
        match (was_active, self.tracker.is_active()) {
            (false, true) => self.activate(qh),
            (true, false) => self.deactivate(),
            _ => {}
        }
    }

    fn activate(&mut self, qh: &QueueHandle<Self>) {
        self.composer.focus_in();
        println!("input method active");
        self.grab = Some(self.input_method.grab_keyboard(qh, ()));
        println!("keyboard grab ready");
    }

    fn deactivate(&mut self) {
        if let Some(grab) = self.grab.take() {
            grab.release();
        }
        self.forwarded.clear();
        let steps = self.composer.focus_out();
        self.send(&steps);
    }

    fn load_keymap(&mut self, fd: OwnedFd, size: u32) {
        let Some(text) = keys::read_text(fd, size) else {
            eprintln!("error: cannot read the grabbed keymap");
            return;
        };
        let Some((file, size)) = keys::write_text(&text) else {
            eprintln!("error: cannot write the keymap for the keyboard");
            return;
        };
        self.keyboard
            .keymap(xkb::KEYMAP_FORMAT_TEXT_V1, file.as_fd(), size);
        self.keymap_file = file;
        self.keymap = Keymap::compile(text);
        if self.keymap.is_some() {
            println!("keymap ready");
        } else {
            eprintln!("error: xkb rejected the grabbed keymap");
        }
    }

    fn set_modifiers(&mut self, depressed: u32, latched: u32, locked: u32, group: u32) {
        if let Some(keymap) = self.keymap.as_mut() {
            keymap.set_modifiers(depressed, latched, locked, group);
        }
        self.keyboard.modifiers(depressed, latched, locked, group);
    }

    fn key(&mut self, time: u32, code: u32, pressed: bool) {
        if !pressed {
            if self.forwarded.remove(&code) {
                self.keyboard.key(time, code, 0);
            }
            return;
        }
        let Some(keymap) = self.keymap.as_ref() else {
            println!("key forwarded without a keymap");
            self.keyboard.key(time, code, 1);
            return;
        };
        let event = keymap.key_event(code);
        println!("diag key {code} {event:?}");
        let (consumed, steps) = self.composer.key(event);
        println!("key consumed: {consumed} steps {steps:?}");
        if consumed {
            self.send(&steps);
        } else {
            self.forwarded.insert(code);
            self.keyboard.key(time, code, 1);
        }
    }

    fn send(&self, steps: &[Step]) {
        if steps.is_empty() {
            return;
        }
        for step in steps {
            match step {
                Step::Preedit(text) => {
                    let cursor = i32::try_from(text.len()).unwrap_or(i32::MAX);
                    self.input_method
                        .set_preedit_string(text.clone(), cursor, cursor);
                }
                Step::Commit(text) => {
                    println!("commit sent: {} bytes", text.len());
                    self.input_method.commit_string(text.clone());
                }
                Step::DeleteBefore(bytes) => self.input_method.delete_surrounding_text(*bytes, 0),
            }
        }
        self.input_method.commit(self.tracker.dones());
    }
}

/// Connects, binds the protocols, and serves the grab until it ends.
pub fn run() -> ExitCode {
    let Some(composer) = Composer::open() else {
        return ExitCode::FAILURE;
    };
    let Ok(connection) = Connection::connect_to_env() else {
        eprintln!("error: cannot connect to the Wayland display");
        return ExitCode::FAILURE;
    };
    let Ok((globals, mut queue)) = registry_queue_init::<Probe>(&connection) else {
        eprintln!("error: cannot read the Wayland globals");
        return ExitCode::FAILURE;
    };
    let handle = queue.handle();
    let Ok(manager) = globals.bind::<ZwpInputMethodManagerV2, _, _>(&handle, 1..=1, ()) else {
        eprintln!("error: the compositor has no input-method-v2 manager");
        return ExitCode::FAILURE;
    };
    let Ok(keyboards) = globals.bind::<ZwpVirtualKeyboardManagerV1, _, _>(&handle, 1..=1, ())
    else {
        eprintln!("error: the compositor has no virtual keyboard manager");
        return ExitCode::FAILURE;
    };
    let Ok(seat) = globals.bind::<WlSeat, _, _>(&handle, 1..=1, ()) else {
        eprintln!("error: the compositor has no seat");
        return ExitCode::FAILURE;
    };
    let Some(text) = keys::default_text() else {
        eprintln!("error: cannot build the default keymap");
        return ExitCode::FAILURE;
    };
    let Some((keymap_file, size)) = keys::write_text(&text) else {
        eprintln!("error: cannot write the default keymap");
        return ExitCode::FAILURE;
    };
    let keyboard = keyboards.create_virtual_keyboard(&seat, &handle, ());
    keyboard.keymap(xkb::KEYMAP_FORMAT_TEXT_V1, keymap_file.as_fd(), size);
    let input_method = manager.get_input_method(&seat, &handle, ());
    let mut probe = Probe {
        tracker: Tracker::default(),
        composer,
        input_method,
        keyboard,
        grab: None,
        keymap: None,
        keymap_file,
        forwarded: HashSet::new(),
        unavailable: false,
    };
    if queue.roundtrip(&mut probe).is_err() {
        eprintln!("error: the Wayland round trip failed");
        return ExitCode::FAILURE;
    }
    println!("input method bound; done events: {}", probe.tracker.dones());
    loop {
        if queue.blocking_dispatch(&mut probe).is_err() {
            eprintln!("error: the Wayland connection failed");
            return ExitCode::FAILURE;
        }
        if probe.unavailable {
            eprintln!("error: another input method is active on this seat");
            return ExitCode::FAILURE;
        }
    }
}
