"""ctypes tests for the keymagic_core C ABI.

Run after `cargo build -p keymagic-ffi`; set KEYMAGIC_CORE_LIB to override.
"""

import ctypes
import os
import platform
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OK = 0
ERR_PARAM = -2
ERR_NO_KEYBOARD = -5
ERR_COMPILE = -6
KMS_SOURCE = (
    b'/*\n@NAME = "Good"\n*/\n$v = "a"\n"a" => "b"\n'
)


def library_path():
    override = os.environ.get("KEYMAGIC_CORE_LIB")
    if override:
        return Path(override)
    names = {
        "Windows": "keymagic_core.dll",
        "Darwin": "libkeymagic_core.dylib",
    }
    name = names.get(platform.system(), "libkeymagic_core.so")
    for profile in ("debug", "release"):
        candidate = ROOT / "target" / profile / name
        if candidate.exists():
            return candidate
    raise FileNotFoundError(f"build keymagic-ffi first: {name} not found")


class HotkeyInfo(ctypes.Structure):
    _fields_ = [
        ("key_code", ctypes.c_int),
        ("ctrl", ctypes.c_int),
        ("alt", ctypes.c_int),
        ("shift", ctypes.c_int),
        ("meta", ctypes.c_int),
    ]


class ProcessKeyOutput(ctypes.Structure):
    _fields_ = [
        ("action_type", ctypes.c_int),
        ("text", ctypes.c_void_p),
        ("delete_count", ctypes.c_int),
        ("composing_text", ctypes.c_void_p),
        ("is_processed", ctypes.c_int),
    ]


LIB = ctypes.CDLL(str(library_path()))


def declare(name, restype, *argtypes):
    function = getattr(LIB, name)
    function.restype = restype
    function.argtypes = list(argtypes)
    return function


engine_new = declare("keymagic_engine_new", ctypes.c_void_p)
engine_free = declare("keymagic_engine_free", None, ctypes.c_void_p)
load_path = declare(
    "keymagic_engine_load_keyboard",
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_char_p,
)
load_memory = declare(
    "keymagic_engine_load_keyboard_from_memory",
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.c_size_t,
)
process_key = declare(
    "keymagic_engine_process_key",
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_int,
    ctypes.c_char,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.POINTER(ProcessKeyOutput),
)
get_composition = declare(
    "keymagic_engine_get_composition", ctypes.c_void_p, ctypes.c_void_p
)
set_composition = declare(
    "keymagic_engine_set_composition",
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_char_p,
)
reset = declare("keymagic_engine_reset", ctypes.c_int, ctypes.c_void_p)
free_string = declare("keymagic_free_string", None, ctypes.c_void_p)
free_bytes = declare(
    "keymagic_free_bytes", None, ctypes.c_void_p, ctypes.c_size_t
)
get_version = declare("keymagic_get_version", ctypes.c_char_p)
km2_load = declare("keymagic_km2_load", ctypes.c_void_p, ctypes.c_char_p)
km2_free = declare("keymagic_km2_free", None, ctypes.c_void_p)
km2_get_name = declare(
    "keymagic_km2_get_name", ctypes.c_void_p, ctypes.c_void_p
)
parse_hotkey = declare(
    "keymagic_parse_hotkey",
    ctypes.c_int,
    ctypes.c_char_p,
    ctypes.POINTER(HotkeyInfo),
)
process_key_win = declare(
    "keymagic_engine_process_key_win",
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_int,
    ctypes.c_char,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.POINTER(ProcessKeyOutput),
)
virtual_key_name = declare(
    "keymagic_virtual_key_to_string", ctypes.c_void_p, ctypes.c_int
)
kms_compile = declare(
    "keymagic_kms_compile",
    ctypes.c_int,
    ctypes.c_char_p,
    ctypes.c_size_t,
    ctypes.POINTER(ctypes.c_void_p),
    ctypes.POINTER(ctypes.c_size_t),
    ctypes.POINTER(ctypes.c_void_p),
)


def take_string(pointer):
    if not pointer:
        return None
    text = ctypes.cast(pointer, ctypes.c_char_p).value.decode("utf-8")
    free_string(pointer)
    return text


def corpus_km2(name):
    return ROOT / "corpus" / "km2" / "alternatives" / name


class FfiTests(unittest.TestCase):
    def test_version_is_the_package_version(self):
        self.assertEqual(get_version(), b"0.1.0")

    def test_null_handles_are_rejected(self):
        output = ProcessKeyOutput()
        self.assertEqual(
            process_key(None, 0, b"\0", 0, 0, 0, 0, ctypes.byref(output)),
            ERR_PARAM,
        )
        self.assertEqual(reset(None), ERR_PARAM)
        self.assertIsNone(get_composition(None))
        free_string(None)
        free_bytes(None, 0)

    def test_engine_without_keyboard_reports_it(self):
        engine = engine_new()
        output = ProcessKeyOutput()
        result = process_key(engine, 0, b"a", 0, 0, 0, 0, ctypes.byref(output))
        self.assertEqual(result, ERR_NO_KEYBOARD)
        engine_free(engine)

    def test_karen_digit_is_inserted_from_a_file(self):
        engine = engine_new()
        path = str(corpus_km2("Eastern-Pwo_kjp.km2")).encode()
        self.assertEqual(load_path(engine, path), OK)
        output = ProcessKeyOutput()
        result = process_key(
            engine, 17, b"1", 0, 0, 0, 0, ctypes.byref(output)
        )
        self.assertEqual(result, OK)
        self.assertEqual(output.action_type, 1)
        self.assertEqual(output.is_processed, 1)
        self.assertEqual(take_string(output.text), "\u1041")
        self.assertEqual(take_string(output.composing_text), "\u1041")
        engine_free(engine)

    def test_memory_load_matches_file_load(self):
        data = corpus_km2("Eastern-Pwo_kjp.km2").read_bytes()
        engine = engine_new()
        self.assertEqual(load_memory(engine, data, len(data)), OK)
        self.assertEqual(load_memory(engine, data, 0), ERR_PARAM)
        engine_free(engine)

    def test_composition_round_trips_and_resets(self):
        engine = engine_new()
        data = corpus_km2("Eastern-Pwo_kjp.km2").read_bytes()
        load_memory(engine, data, len(data))
        self.assertEqual(set_composition(engine, "abc".encode()), OK)
        self.assertEqual(take_string(get_composition(engine)), "abc")
        self.assertEqual(reset(engine), OK)
        self.assertEqual(take_string(get_composition(engine)), "")
        engine_free(engine)

    def test_km2_metadata_is_a_string_or_absent(self):
        handle = km2_load(str(corpus_km2("Eastern-Pwo_kjp.km2")).encode())
        self.assertTrue(handle)
        name = take_string(km2_get_name(handle))
        self.assertTrue(name is None or isinstance(name, str))
        km2_free(handle)

    def test_kms_compile_round_trips_through_the_engine(self):
        out_bytes = ctypes.c_void_p()
        out_len = ctypes.c_size_t()
        out_diag = ctypes.c_void_p()
        result = kms_compile(
            KMS_SOURCE,
            len(KMS_SOURCE),
            ctypes.byref(out_bytes),
            ctypes.byref(out_len),
            ctypes.byref(out_diag),
        )
        self.assertEqual(result, OK)
        self.assertIsNone(out_diag.value)
        data = ctypes.string_at(out_bytes.value, out_len.value)
        free_bytes(out_bytes.value, out_len.value)
        engine = engine_new()
        self.assertEqual(load_memory(engine, data, len(data)), OK)
        output = ProcessKeyOutput()
        process_key(engine, 0, b"a", 0, 0, 0, 0, ctypes.byref(output))
        self.assertEqual(take_string(output.composing_text), "b")
        take_string(output.text)
        engine_free(engine)

    def test_windows_key_codes_are_checked(self):
        output = ProcessKeyOutput()
        bad = process_key_win(
            engine_new(), 0xFFFF, b"\0", 0, 0, 0, 0, ctypes.byref(output)
        )
        self.assertEqual(bad, ERR_PARAM)
        engine = engine_new()
        good = process_key_win(
            engine, 0x41, b"a", 0, 0, 0, 0, ctypes.byref(output)
        )
        self.assertEqual(good, ERR_NO_KEYBOARD)
        engine_free(engine)

    def test_hotkey_parses_modifiers_and_rejects_two_keys(self):
        info = HotkeyInfo()
        self.assertEqual(parse_hotkey(b"Ctrl+Shift+A", ctypes.byref(info)), 1)
        self.assertEqual((info.ctrl, info.shift, info.alt), (1, 1, 0))
        self.assertEqual(parse_hotkey(b"Ctrl+A+B", ctypes.byref(info)), 0)

    def test_virtual_key_names_resolve_or_are_absent(self):
        self.assertIsNotNone(take_string(virtual_key_name(17)))
        self.assertIsNone(virtual_key_name(-1))

    def test_kms_compile_reports_diagnostics_as_json(self):
        source = b"$a = $b\n"
        out_bytes = ctypes.c_void_p()
        out_len = ctypes.c_size_t()
        out_diag = ctypes.c_void_p()
        result = kms_compile(
            source,
            len(source),
            ctypes.byref(out_bytes),
            ctypes.byref(out_len),
            ctypes.byref(out_diag),
        )
        self.assertEqual(result, ERR_COMPILE)
        self.assertFalse(out_bytes.value)
        diagnostics = take_string(out_diag.value)
        self.assertIn('"severity":"error"', diagnostics)


if __name__ == "__main__":
    unittest.main()
