//! Binds input-method-v2, adds a virtual keyboard, and commits text.

use crate::tracker::{Surrounding, Tracker};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::AsFd;
use std::process::ExitCode;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
};
use wayland_protocols_misc::zwp_input_method_v2::client::{
    zwp_input_method_manager_v2::{self, ZwpInputMethodManagerV2},
    zwp_input_method_v2::{self, ZwpInputMethodV2},
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::{self, ZwpVirtualKeyboardManagerV1},
    zwp_virtual_keyboard_v1::{self, ZwpVirtualKeyboardV1},
};
use xkbcommon::xkb;

/// Text the probe commits into the focused text input.
const PROBE_TEXT: &str = "ခ";

/// Protocol state for one probe run.
#[derive(Default)]
struct Probe {
    tracker: Tracker,
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
        _: &QueueHandle<Self>,
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
            zwp_input_method_v2::Event::Done => state.tracker.done(),
            zwp_input_method_v2::Event::Unavailable => state.unavailable = true,
            _ => {}
        }
    }
}

/// Connects, binds the protocols, commits text after activation, and reports.
pub fn run() -> ExitCode {
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
    let Some((keymap, size)) = keymap_file() else {
        eprintln!("error: cannot write the default keymap");
        return ExitCode::FAILURE;
    };
    let keyboard = keyboards.create_virtual_keyboard(&seat, &handle, ());
    keyboard.keymap(xkb::KEYMAP_FORMAT_TEXT_V1, keymap.as_fd(), size);
    let input_method = manager.get_input_method(&seat, &handle, ());
    let mut probe = Probe::default();
    if queue.roundtrip(&mut probe).is_err() {
        eprintln!("error: the Wayland round trip failed");
        return ExitCode::FAILURE;
    }
    if probe.unavailable {
        eprintln!("error: another input method is active on this seat");
        return ExitCode::FAILURE;
    }
    println!("input method bound; done events: {}", probe.tracker.dones());
    if !wait_for_activation(&mut queue, &mut probe) {
        eprintln!("error: no text input activated the input method");
        return ExitCode::FAILURE;
    }
    input_method.commit_string(PROBE_TEXT.to_owned());
    input_method.commit(probe.tracker.dones());
    if queue.roundtrip(&mut probe).is_err() {
        eprintln!("error: the commit round trip failed");
        return ExitCode::FAILURE;
    }
    println!("committed {PROBE_TEXT} after activation");
    ExitCode::SUCCESS
}

/// Writes the default keymap to an unlinked file, with a NUL terminator.
fn keymap_file() -> Option<(File, u32)> {
    let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        "",
        "",
        None,
        xkb::KEYMAP_COMPILE_NO_FLAGS,
    )?;
    let mut bytes = keymap
        .get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1)
        .into_bytes();
    bytes.push(0);
    let path = std::env::temp_dir().join(format!("keymagic-keymap-{}.xkb", std::process::id()));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .ok()?;
    fs::remove_file(&path).ok()?;
    file.write_all(&bytes).ok()?;
    Some((file, u32::try_from(bytes.len()).ok()?))
}

/// Dispatches until activation; `false` if dispatching fails.
fn wait_for_activation(queue: &mut EventQueue<Probe>, probe: &mut Probe) -> bool {
    while !probe.tracker.is_active() {
        if queue.blocking_dispatch(probe).is_err() {
            return false;
        }
    }
    true
}
