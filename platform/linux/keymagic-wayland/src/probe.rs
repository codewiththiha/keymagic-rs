//! Binds an input-method protocol, grabs the keyboard, and sends engine output.

use crate::engine::{Composer, Step};
use crate::keys::{self, Keymap};
use crate::tracker::{Surrounding, Tracker};
use crate::wire::{Passed, v1_delete_index};
use std::fs::File;
use std::os::fd::{AsFd, OwnedFd};
use std::process::ExitCode;
use wayland_client::{
    Connection, Dispatch, QueueHandle, WEnum,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_keyboard::{self, WlKeyboard},
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
};
use wayland_protocols::wp::input_method::zv1::client::{
    zwp_input_method_context_v1::{self, ZwpInputMethodContextV1},
    zwp_input_method_v1::{self, ZwpInputMethodV1},
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

/// The input-method-v2 session: the method, a virtual keyboard, and a grab.
struct V2 {
    input_method: ZwpInputMethodV2,
    keyboard: ZwpVirtualKeyboardV1,
    grab: Option<ZwpInputMethodKeyboardGrabV2>,
}

/// The input-method-v1 session, used when the compositor offers only v1.
struct V1 {
    _global: ZwpInputMethodV1,
    context: Option<ZwpInputMethodContextV1>,
    serial: u32,
    keyboard: Option<WlKeyboard>,
}

/// Protocol state for one probe run.
struct Probe {
    tracker: Tracker,
    composer: Composer,
    keymap: Option<Keymap>,
    keymap_file: File,
    passed: Passed,
    unavailable: bool,
    v2: Option<V2>,
    v1: Option<V1>,
}

fn pressed(state: WEnum<wl_keyboard::KeyState>) -> bool {
    matches!(state, WEnum::Value(wl_keyboard::KeyState::Pressed))
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
                serial,
                time,
                key,
                state: key_state,
            } => state.key(serial, time, key, pressed(key_state)),
            zwp_input_method_keyboard_grab_v2::Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
            } => state.set_modifiers(serial, mods_depressed, mods_latched, mods_locked, group),
            _ => {}
        }
    }
}

impl Dispatch<ZwpInputMethodV1, ()> for Probe {
    fn event(
        state: &mut Self,
        _: &ZwpInputMethodV1,
        event: zwp_input_method_v1::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_input_method_v1::Event::Activate { id } => state.activate_v1(id, qh),
            zwp_input_method_v1::Event::Deactivate { .. } => state.deactivate_v1(),
            _ => {}
        }
    }

    wayland_client::event_created_child!(Probe, ZwpInputMethodV1, [
        0 => (ZwpInputMethodContextV1, ())
    ]);
}

impl Dispatch<ZwpInputMethodContextV1, ()> for Probe {
    fn event(
        state: &mut Self,
        _: &ZwpInputMethodContextV1,
        event: zwp_input_method_context_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_input_method_context_v1::Event::CommitState { serial } = event
            && let Some(v1) = state.v1.as_mut()
        {
            v1.serial = serial;
        }
    }
}

impl Dispatch<WlKeyboard, ()> for Probe {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Keymap { fd, size, .. } => state.load_keymap(fd, size),
            wl_keyboard::Event::Key {
                serial,
                time,
                key,
                state: key_state,
                ..
            } => state.key(serial, time, key, pressed(key_state)),
            wl_keyboard::Event::Modifiers {
                serial,
                mods_depressed,
                mods_latched,
                mods_locked,
                group,
                ..
            } => state.set_modifiers(serial, mods_depressed, mods_latched, mods_locked, group),
            _ => {}
        }
    }
}

impl Probe {
    fn done(&mut self, qh: &QueueHandle<Self>) {
        let was_active = self.tracker.is_active();
        self.tracker.done();
        match (was_active, self.tracker.is_active()) {
            (false, true) => self.activate_v2(qh),
            (true, false) => self.deactivate_v2(),
            _ => {}
        }
    }

    fn activate_v2(&mut self, qh: &QueueHandle<Self>) {
        self.composer.focus_in();
        println!("input method active");
        if let Some(v2) = self.v2.as_mut() {
            v2.grab = Some(v2.input_method.grab_keyboard(qh, ()));
        }
        println!("keyboard grab ready");
    }

    fn deactivate_v2(&mut self) {
        let grab = self.v2.as_mut().and_then(|v2| v2.grab.take());
        if let Some(grab) = grab {
            grab.release();
        }
        self.finish_session();
    }

    fn activate_v1(&mut self, context: ZwpInputMethodContextV1, qh: &QueueHandle<Self>) {
        self.composer.focus_in();
        println!("input method active");
        let keyboard = context.grab_keyboard(qh, ());
        if let Some(v1) = self.v1.as_mut() {
            v1.context = Some(context);
            v1.keyboard = Some(keyboard);
        }
        println!("keyboard grab ready");
    }

    fn deactivate_v1(&mut self) {
        let Some(v1) = self.v1.as_mut() else {
            return;
        };
        let keyboard = v1.keyboard.take();
        let context = v1.context.take();
        if let Some(keyboard) = keyboard {
            keyboard.release();
        }
        self.finish_session();
        if let Some(context) = context {
            context.destroy();
        }
    }

    /// Flushes pending text and forgets the keys sent to the application.
    fn finish_session(&mut self) {
        self.passed.clear();
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
        if let Some(v2) = self.v2.as_ref() {
            v2.keyboard
                .keymap(xkb::KEYMAP_FORMAT_TEXT_V1, file.as_fd(), size);
        }
        self.keymap_file = file;
        self.keymap = Keymap::compile(text);
        if self.keymap.is_some() {
            println!("keymap ready");
        } else {
            eprintln!("error: xkb rejected the grabbed keymap");
        }
    }

    fn set_modifiers(
        &mut self,
        serial: u32,
        depressed: u32,
        latched: u32,
        locked: u32,
        group: u32,
    ) {
        if let Some(keymap) = self.keymap.as_mut() {
            keymap.set_modifiers(depressed, latched, locked, group);
        }
        match (&self.v2, &self.v1) {
            (Some(v2), _) => v2.keyboard.modifiers(depressed, latched, locked, group),
            (
                None,
                Some(V1 {
                    context: Some(context),
                    ..
                }),
            ) => {
                context.modifiers(serial, depressed, latched, locked, group);
            }
            _ => {}
        }
    }

    fn key(&mut self, serial: u32, time: u32, code: u32, pressed: bool) {
        if !pressed {
            let forwarded = self.passed.release(code);
            if forwarded {
                self.forward_key(serial, time, code, 0);
            }
            return;
        }
        let (consumed, steps) = match self.keymap.as_ref() {
            Some(keymap) => self.composer.key(keymap.key_event(code)),
            None => (false, Vec::new()),
        };
        if consumed {
            self.send(&steps);
        } else {
            self.passed.press(code);
            self.forward_key(serial, time, code, 1);
        }
    }

    fn forward_key(&self, serial: u32, time: u32, code: u32, state: u32) {
        match (&self.v2, &self.v1) {
            (Some(v2), _) => v2.keyboard.key(time, code, state),
            (
                None,
                Some(V1 {
                    context: Some(context),
                    ..
                }),
            ) => {
                context.key(serial, time, code, state);
            }
            _ => {}
        }
    }

    fn send(&self, steps: &[Step]) {
        if steps.is_empty() {
            return;
        }
        match (&self.v2, &self.v1) {
            (Some(v2), _) => send_v2(&v2.input_method, self.tracker.dones(), steps),
            (
                None,
                Some(V1 {
                    context: Some(context),
                    serial,
                    ..
                }),
            ) => {
                send_v1(context, *serial, steps);
            }
            _ => {}
        }
    }
}

fn send_v2(input_method: &ZwpInputMethodV2, dones: u32, steps: &[Step]) {
    for step in steps {
        match step {
            Step::Preedit(text) => {
                let cursor = i32::try_from(text.len()).unwrap_or(i32::MAX);
                input_method.set_preedit_string(text.clone(), cursor, cursor);
            }
            Step::Commit(text) => {
                println!("commit sent: {} bytes", text.len());
                input_method.commit_string(text.clone());
            }
            Step::DeleteBefore(bytes) => input_method.delete_surrounding_text(*bytes, 0),
        }
    }
    input_method.commit(dones);
}

fn send_v1(context: &ZwpInputMethodContextV1, serial: u32, steps: &[Step]) {
    for step in steps {
        match step {
            Step::Preedit(text) => context.preedit_string(serial, text.clone(), String::new()),
            Step::Commit(text) => {
                println!("commit sent: {} bytes", text.len());
                context.commit_string(serial, text.clone());
            }
            Step::DeleteBefore(bytes) => {
                context.delete_surrounding_text(v1_delete_index(*bytes), *bytes);
            }
        }
    }
}

/// Binds the input-method-v2 pieces; `None` after printing the error.
fn bind_v2(
    globals: &GlobalList,
    handle: &QueueHandle<Probe>,
    manager: &ZwpInputMethodManagerV2,
    keymap: &File,
    size: u32,
) -> Option<V2> {
    let seat: WlSeat = globals.bind(handle, 1..=1, ()).ok()?;
    let keyboards: ZwpVirtualKeyboardManagerV1 = globals.bind(handle, 1..=1, ()).ok()?;
    let keyboard = keyboards.create_virtual_keyboard(&seat, handle, ());
    keyboard.keymap(xkb::KEYMAP_FORMAT_TEXT_V1, keymap.as_fd(), size);
    Some(V2 {
        input_method: manager.get_input_method(&seat, handle, ()),
        keyboard,
        grab: None,
    })
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
    let Some(text) = keys::default_text() else {
        eprintln!("error: cannot build the default keymap");
        return ExitCode::FAILURE;
    };
    let Some((keymap_file, size)) = keys::write_text(&text) else {
        eprintln!("error: cannot write the default keymap");
        return ExitCode::FAILURE;
    };
    let mut probe = Probe {
        tracker: Tracker::default(),
        composer,
        keymap: None,
        keymap_file,
        passed: Passed::default(),
        unavailable: false,
        v2: None,
        v1: None,
    };
    if let Ok(manager) = globals.bind::<ZwpInputMethodManagerV2, _, _>(&handle, 1..=1, ()) {
        let Some(v2) = bind_v2(&globals, &handle, &manager, &probe.keymap_file, size) else {
            return ExitCode::FAILURE;
        };
        probe.v2 = Some(v2);
        println!("protocol: input-method-v2");
    } else if let Ok(global) = globals.bind::<ZwpInputMethodV1, _, _>(&handle, 1..=1, ()) {
        probe.v1 = Some(V1 {
            _global: global,
            context: None,
            serial: 0,
            keyboard: None,
        });
        println!("protocol: input-method-v1");
    } else {
        eprintln!("error: the compositor has no input method protocol");
        return ExitCode::FAILURE;
    }
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
