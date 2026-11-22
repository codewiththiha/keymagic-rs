"""ctypes tests for the keymagic_core C ABI, ported from KM3's test_ffi.py.

Run after `cargo build -p keymagic-ffi`; set KEYMAGIC_CORE_LIB to override.
"""

import ctypes
import os
import platform
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OK = 0
ERR_PARAM = -2
ERR_ENGINE = -3
ERR_NO_KEYBOARD = -5
ERR_COMPILE = -6

VK_BACK = 2
VK_KEY_A = 26
VK_KEY_K = 36
WIN_VK_A = 0x41
WIN_VK_BACK = 0x08

KMS_SOURCE = (
    b'/*\n@NAME = "Good"\n*/\n$v = "a"\n"a" => "b"\n'
)

# The keyboard that KM3's test_ffi.py compiles before its engine tests.
UPSTREAM_KMS = """/*
@NAME = "Test Keyboard"
@DESCRIPTION = "Simple test keyboard for FFI testing"
@TRACK_CAPSLOCK = "FALSE"
@SMART_BACKSPACE = "TRUE"
@US_LAYOUT_BASED = "TRUE"
*/

// Simple rules for testing
"a" => "အ"
"ka" => "က"
"kha" => "ခ"

// Test with modifier
<VK_SHIFT & VK_KEY_A> => "အ"

// Test backspace
"က" + <VK_BACK> => NULL
""".encode("utf-8")


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


def declare_process(name):
    return declare(
        name,
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


engine_new = declare("keymagic_engine_new", ctypes.c_void_p)
engine_free = declare("keymagic_engine_free", None, ctypes.c_void_p)
load_path = declare(
    "keymagic_engine_load_keyboard", ctypes.c_int, ctypes.c_void_p, ctypes.c_char_p
)
load_memory = declare(
    "keymagic_engine_load_keyboard_from_memory",
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_char_p,
    ctypes.c_size_t,
)
process_key = declare_process("keymagic_engine_process_key")
process_key_test = declare_process("keymagic_engine_process_key_test")
process_key_win = declare_process("keymagic_engine_process_key_win")
process_key_test_win = declare_process("keymagic_engine_process_key_test_win")
get_composition = declare(
    "keymagic_engine_get_composition", ctypes.c_void_p, ctypes.c_void_p
)
get_composing_text = declare(
    "keymagic_engine_get_composing_text", ctypes.c_void_p, ctypes.c_void_p
)
engine_free_string = declare("keymagic_engine_free_string", None, ctypes.c_void_p)
set_composition = declare(
    "keymagic_engine_set_composition", ctypes.c_int, ctypes.c_void_p, ctypes.c_char_p
)
reset = declare("keymagic_engine_reset", ctypes.c_int, ctypes.c_void_p)
free_string = declare("keymagic_free_string", None, ctypes.c_void_p)
free_bytes = declare("keymagic_free_bytes", None, ctypes.c_void_p, ctypes.c_size_t)
get_version = declare("keymagic_get_version", ctypes.c_char_p)
km2_load = declare("keymagic_km2_load", ctypes.c_void_p, ctypes.c_char_p)
km2_free = declare("keymagic_km2_free", None, ctypes.c_void_p)
km2_get_name = declare("keymagic_km2_get_name", ctypes.c_void_p, ctypes.c_void_p)
km2_get_description = declare(
    "keymagic_km2_get_description", ctypes.c_void_p, ctypes.c_void_p
)
parse_hotkey = declare(
    "keymagic_parse_hotkey",
    ctypes.c_int,
    ctypes.c_char_p,
    ctypes.POINTER(HotkeyInfo),
)
virtual_key_name = declare("keymagic_virtual_key_to_string", ctypes.c_void_p, ctypes.c_int)
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


def compile_kms(source):
    data = ctypes.c_void_p()
    length = ctypes.c_size_t()
    diagnostics = ctypes.c_void_p()
    result = kms_compile(
        source,
        len(source),
        ctypes.byref(data),
        ctypes.byref(length),
        ctypes.byref(diagnostics),
    )
    if result != OK:
        raise AssertionError(f"compile failed with {result}: {take_string(diagnostics.value)}")
    km2 = ctypes.string_at(data.value, length.value)
    free_bytes(data.value, length.value)
    return km2


def press(function, engine, key_code, character=b"\0", flags=(0, 0, 0, 0)):
    output = ProcessKeyOutput()
    result = function(engine, key_code, character, *flags, ctypes.byref(output))
    return {
        "result": result,
        "action": output.action_type,
        "delete": output.delete_count,
        "processed": output.is_processed,
        "text": take_string(output.text),
        "composing": take_string(output.composing_text),
    }


def corpus_km2(name):
    return ROOT / "corpus" / "km2" / "alternatives" / name


class FfiTests(unittest.TestCase):
    def engine_with(self, km2):
        engine = engine_new()
        self.addCleanup(engine_free, engine)
        self.assertEqual(load_memory(engine, km2, len(km2)), OK)
        return engine

    def write_temp(self, data):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "keyboard.km2"
        path.write_bytes(data)
        return path

    def test_version_is_the_package_version(self):
        self.assertEqual(get_version(), b"0.1.0")

    def test_engine_lifecycle(self):
        engine = engine_new()
        self.assertTrue(engine)
        engine_free(engine)

    def test_null_handles_are_rejected(self):
        output = ProcessKeyOutput()
        self.assertEqual(
            process_key(None, 0, b"\0", 0, 0, 0, 0, ctypes.byref(output)),
            ERR_PARAM,
        )
        self.assertEqual(process_key_win(None, 0x41, b"a", 0, 0, 0, 0, ctypes.byref(output)), ERR_PARAM)
        self.assertEqual(reset(None), ERR_PARAM)
        self.assertEqual(set_composition(None, b"a"), ERR_PARAM)
        self.assertEqual(load_memory(None, b"x", 1), ERR_PARAM)
        self.assertEqual(load_path(None, b"test.km2"), ERR_PARAM)
        self.assertIsNone(get_composition(None))
        free_string(None)
        free_bytes(None, 0)

    def test_null_paths_are_rejected(self):
        engine = engine_new()
        self.addCleanup(engine_free, engine)
        self.assertEqual(load_path(engine, None), ERR_PARAM)
        self.assertEqual(load_memory(engine, UPSTREAM_KMS, 0), ERR_PARAM)

    def test_missing_file_is_an_engine_failure(self):
        engine = engine_new()
        self.addCleanup(engine_free, engine)
        self.assertEqual(load_path(engine, b"nonexistent.km2"), ERR_ENGINE)

    def test_engine_without_keyboard_reports_it(self):
        engine = engine_new()
        self.addCleanup(engine_free, engine)
        result = press(process_key, engine, 0, b"a")["result"]
        self.assertEqual(result, ERR_NO_KEYBOARD)

    def test_compiled_file_loads_from_a_path(self):
        path = self.write_temp(compile_kms(UPSTREAM_KMS))
        engine = engine_new()
        self.addCleanup(engine_free, engine)
        self.assertEqual(load_path(engine, str(path).encode()), OK)

    def test_memory_loading(self):
        km2 = compile_kms(UPSTREAM_KMS)
        self.engine_with(km2)

    def test_key_processing_inserts_utf8_text(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        outcome = press(process_key, engine, 65, b"a")
        self.assertEqual(outcome["result"], OK)
        self.assertEqual(outcome["processed"], 1)
        self.assertEqual(outcome["action"], 1)
        self.assertEqual(outcome["text"], "အ")
        self.assertEqual(list("အ".encode("utf-8")), [0xE1, 0x80, 0xA1])

    def test_multi_char_sequence_replaces_the_prefix(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        first = press(process_key, engine, 75, b"k")
        self.assertEqual(first["composing"], "k")
        second = press(process_key, engine, 65, b"a")
        self.assertEqual(second["processed"], 1)
        self.assertEqual(second["action"], 3)
        self.assertEqual(second["delete"], 1)
        self.assertEqual(second["text"], "က")

    def test_engine_reset_clears_the_composition(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        press(process_key, engine, 75, b"k")
        self.assertEqual(reset(engine), OK)
        self.assertEqual(take_string(get_composition(engine)), "")

    def test_memory_safety_over_repeated_engines(self):
        km2 = compile_kms(UPSTREAM_KMS)
        for _ in range(10):
            engine = self.engine_with(km2)
            for character in "hello":
                outcome = press(process_key, engine, ord(character.upper()), character.encode())
                self.assertEqual(outcome["result"], OK)

    def test_peek_leaves_the_engine_unchanged(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        peeked = press(process_key_test, engine, 75, b"k")
        self.assertEqual(peeked["composing"], "k")
        self.assertEqual(take_string(get_composition(engine)), "")

    def test_composing_text_is_freed_by_its_own_call(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        press(process_key, engine, 75, b"k")
        pointer = get_composing_text(engine)
        self.assertEqual(ctypes.cast(pointer, ctypes.c_char_p).value, b"k")
        engine_free_string(pointer)

    def test_windows_key_codes_map_to_virtual_keys(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        inserted = press(process_key_win, engine, WIN_VK_A, b"a")
        self.assertEqual(inserted["result"], OK)
        self.assertEqual(inserted["action"], 1)
        self.assertEqual(inserted["text"], "အ")
        self.assertEqual(press(process_key_win, engine, WIN_VK_BACK)["result"], OK)

    def test_windows_key_codes_are_checked(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        self.assertEqual(press(process_key_win, engine, 0xFFFF, b"a")["result"], ERR_PARAM)

    def test_windows_key_needs_a_keyboard(self):
        engine = engine_new()
        self.addCleanup(engine_free, engine)
        self.assertEqual(press(process_key_win, engine, WIN_VK_A, b"a")["result"], ERR_NO_KEYBOARD)

    def test_windows_peek_leaves_the_engine_unchanged(self):
        engine = self.engine_with(compile_kms(UPSTREAM_KMS))
        self.assertEqual(press(process_key_test_win, engine, WIN_VK_A, b"a")["result"], OK)
        self.assertEqual(take_string(get_composition(engine)), "")

    def test_hotkey_parses_modifiers_and_rejects_two_keys(self):
        info = HotkeyInfo()
        self.assertEqual(parse_hotkey(b"Ctrl+Shift+A", ctypes.byref(info)), 1)
        self.assertEqual((info.ctrl, info.shift, info.alt, info.meta), (1, 1, 0, 0))
        self.assertEqual(info.key_code, VK_KEY_A)
        self.assertEqual(parse_hotkey(b"Ctrl+A+B", ctypes.byref(info)), 0)

    def test_virtual_key_names_follow_upstream(self):
        self.assertEqual(take_string(virtual_key_name(VK_BACK)), "Backspace")
        self.assertEqual(take_string(virtual_key_name(VK_KEY_A)), "A")
        self.assertEqual(take_string(virtual_key_name(VK_KEY_K)), "K")
        self.assertIsNone(virtual_key_name(1))
        self.assertIsNone(virtual_key_name(-1))
        self.assertIsNone(virtual_key_name(0x10000))

    def test_km2_metadata_reads_the_upstream_keyboard(self):
        path = self.write_temp(compile_kms(UPSTREAM_KMS))
        handle = km2_load(str(path).encode())
        self.assertTrue(handle)
        self.addCleanup(km2_free, handle)
        self.assertEqual(take_string(km2_get_name(handle)), "Test Keyboard")
        self.assertEqual(
            take_string(km2_get_description(handle)),
            "Simple test keyboard for FFI testing",
        )

    def test_karen_digit_is_inserted_from_a_file(self):
        engine = engine_new()
        self.addCleanup(engine_free, engine)
        path = str(corpus_km2("Eastern-Pwo_kjp.km2")).encode()
        self.assertEqual(load_path(engine, path), OK)
        outcome = press(process_key, engine, 17, b"1")
        self.assertEqual(outcome["result"], OK)
        self.assertEqual(outcome["action"], 1)
        self.assertEqual(outcome["processed"], 1)
        self.assertEqual(outcome["text"], "\u1041")
        self.assertEqual(outcome["composing"], "\u1041")

    def test_memory_load_matches_file_load(self):
        data = corpus_km2("Eastern-Pwo_kjp.km2").read_bytes()
        self.engine_with(data)

    def test_composition_round_trips_and_resets(self):
        engine = self.engine_with(corpus_km2("Eastern-Pwo_kjp.km2").read_bytes())
        self.assertEqual(set_composition(engine, b"abc"), OK)
        self.assertEqual(take_string(get_composition(engine)), "abc")
        self.assertEqual(reset(engine), OK)
        self.assertEqual(take_string(get_composition(engine)), "")

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
        engine = self.engine_with(data)
        self.assertEqual(press(process_key, engine, 0, b"a")["composing"], "b")

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
