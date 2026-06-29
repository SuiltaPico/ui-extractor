#ifndef UI_EXTRACTOR_H
#define UI_EXTRACTOR_H

#include <stddef.h>
#include <stdint.h>

#ifdef _WIN32
#  ifdef UI_EXTRACTOR_BUILD
#    define UI_EXTRACTOR_API __declspec(dllexport)
#  else
#    define UI_EXTRACTOR_API __declspec(dllimport)
#  endif
#else
#  define UI_EXTRACTOR_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct UiExtractorHandle UiExtractorHandle;

/* Returns a static version string (do not free). */
UI_EXTRACTOR_API const char *ui_extractor_version(void);

/* Free strings returned by this library. */
UI_EXTRACTOR_API void ui_extractor_string_free(char *s);

/*
 * Create an extractor from JSON config (UTF-8).
 * infer_registry: NULL → open an owned infer-core registry from config
 *                 non-NULL → borrow existing InferRegistry* (not destroyed here)
 * When borrowing, models_dir/runtime in config_json are ignored.
 */
UI_EXTRACTOR_API void *ui_extractor_create(
    void *infer_registry,
    const char *config_json,
    char **out_error);

#define ui_extractor_create_standalone(json, err) \
    ui_extractor_create(NULL, (json), (err))

UI_EXTRACTOR_API void *ui_extractor_create_from_registry(
    void *infer_registry,
    const char *config_json,
    char **out_error);

UI_EXTRACTOR_API void ui_extractor_destroy(void *handle);

/* Extract UI tree as JSON (caller frees `*out_json`). */
UI_EXTRACTOR_API int ui_extractor_extract_bytes(
    void *handle,
    const uint8_t *data, size_t len,
    char **out_json, char **out_error);

UI_EXTRACTOR_API int ui_extractor_extract_file(
    void *handle,
    const char *path,
    char **out_json, char **out_error);

UI_EXTRACTOR_API int ui_extractor_reload_icon_pack(void *handle, char **out_error);

#ifdef __cplusplus
}
#endif

#endif /* UI_EXTRACTOR_H */
