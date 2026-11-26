//! Binds the input-method-v2 manager and seat, then reports the result.

use crate::tracker::{Surrounding, Tracker};
use std::process::ExitCode;
use wayland_client::{
    Connection, Dispatch, QueueHandle,
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
            } => {
                state.tracker.surrounding(Surrounding {
                    text,
                    cursor,
                    anchor,
                });
            }
            zwp_input_method_v2::Event::Done => state.tracker.done(),
            zwp_input_method_v2::Event::Unavailable => state.unavailable = true,
            _ => {}
        }
    }
}

/// Connects, binds the first seat, and reports whether it can be used.
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
    let Ok(seat) = globals.bind::<WlSeat, _, _>(&handle, 1..=1, ()) else {
        eprintln!("error: the compositor has no seat");
        return ExitCode::FAILURE;
    };
    let _input_method = manager.get_input_method(&seat, &handle, ());
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
    ExitCode::SUCCESS
}
