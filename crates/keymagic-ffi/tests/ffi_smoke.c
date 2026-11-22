/* Smoke test of the exported ABI, written against the C declarations. */
#include <stddef.h>
#include <stdint.h>
#include <string.h>

typedef struct KmEngine KmEngine;

typedef struct {
    int action_type;
    char *text;
    int delete_count;
    char *composing_text;
    int is_processed;
} KmProcessKeyOutput;

KmEngine *keymagic_engine_new(void);
void keymagic_engine_free(KmEngine *engine);
int keymagic_engine_load_keyboard_from_memory(KmEngine *engine,
                                              const uint8_t *data, size_t len);
int keymagic_engine_process_key(KmEngine *engine, int key_code, char character,
                                int shift, int ctrl, int alt, int caps_lock,
                                KmProcessKeyOutput *output);
char *keymagic_engine_get_composition(KmEngine *engine);
int keymagic_kms_compile(const uint8_t *src, size_t len, uint8_t **out_bytes,
                         size_t *out_len, char **out_diag_json);
void keymagic_free_string(char *text);
void keymagic_free_bytes(uint8_t *data, size_t len);

int keymagic_ffi_smoke(void) {
    static const char source[] =
        "/*\n@NAME = \"Good\"\n*/\n$v = \"a\"\n\"a\" => \"b\"\n";
    uint8_t *bytes = NULL;
    size_t len = 0;
    char *diag = NULL;
    KmEngine *engine = NULL;
    KmProcessKeyOutput out;
    char *composition = NULL;
    int ok = 0;

    if (keymagic_kms_compile((const uint8_t *)source, strlen(source), &bytes,
                             &len, &diag) != 0) {
        return 1;
    }
    if (diag != NULL || bytes == NULL || len == 0) {
        return 2;
    }
    engine = keymagic_engine_new();
    if (engine == NULL) {
        return 3;
    }
    if (keymagic_engine_load_keyboard_from_memory(engine, bytes, len) != 0) {
        return 4;
    }
    keymagic_free_bytes(bytes, len);
    memset(&out, 0, sizeof out);
    if (keymagic_engine_process_key(engine, 0, 'a', 0, 0, 0, 0, &out) != 0) {
        return 5;
    }
    ok = out.is_processed == 1 && out.composing_text != NULL &&
         strcmp(out.composing_text, "b") == 0;
    keymagic_free_string(out.text);
    keymagic_free_string(out.composing_text);
    composition = keymagic_engine_get_composition(engine);
    ok = ok && composition != NULL && strcmp(composition, "b") == 0;
    keymagic_free_string(composition);
    keymagic_engine_free(engine);
    return ok ? 0 : 6;
}
