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
typedef struct IconPackHandle IconPackHandle;

/* Returns a static version string (do not free). */
UI_EXTRACTOR_API const char *ui_extractor_version(void);

/* Free strings returned by this library. */
UI_EXTRACTOR_API void ui_extractor_string_free(char *s);

/*
 * Create an extractor from JSON config (UTF-8).
 * Returns opaque handle or NULL; on failure `*out_error` is set.
 */
UI_EXTRACTOR_API void *ui_extractor_create(const char *config_json, char **out_error);
UI_EXTRACTOR_API void ui_extractor_destroy(void *handle);

/* Extract UI tree as JSON (caller frees `*out_json`). */
UI_EXTRACTOR_API int ui_extractor_extract_bytes(
    void *handle,
    const uint8_t *data, size_t len,
    char **out_json, char **out_error);

UI_EXTRACTOR_API int ui_extractor_extract_file(
    void *handle,
    const char *path,
    char **out_json,
    char **out_error);

UI_EXTRACTOR_API int ui_extractor_reload_icon_pack(void *handle, char **out_error);

/* Icon pack: load precomputed index. */
UI_EXTRACTOR_API void *ui_icon_pack_load(
    const char *embedding_index,
    const char *vision_model,
    uint32_t template_size,
    double min_cosine,
    char **out_error);

UI_EXTRACTOR_API void ui_icon_pack_destroy(void *handle);

UI_EXTRACTOR_API int ui_icon_pack_embed_image_bytes(
    void *handle,
    const uint8_t *data,
    size_t len,
    float *out_embedding,
    uint32_t dim,
    char **out_error);

/* Match/search return JSON (`{"name":"...","score":0.9}` or `null` / array). */
UI_EXTRACTOR_API int ui_icon_pack_match_embedding(
    void *handle,
    const float *embedding,
    uint32_t dim,
    char **out_json,
    char **out_error);

UI_EXTRACTOR_API int ui_icon_pack_search_embedding(
    void *handle,
    const float *embedding,
    uint32_t dim,
    uint32_t top_k,
    char **out_json,
    char **out_error);

UI_EXTRACTOR_API int ui_icon_pack_match_image_file(
    void *handle,
    const char *path,
    char **out_json,
    char **out_error);

UI_EXTRACTOR_API int ui_icon_pack_match_region_file(
    void *handle,
    const char *path,
    int x,
    int y,
    int width,
    int height,
    char **out_json,
    char **out_error);

UI_EXTRACTOR_API uint32_t ui_icon_embedding_dim(void);

#ifdef __cplusplus
}
#endif

#endif /* UI_EXTRACTOR_H */
