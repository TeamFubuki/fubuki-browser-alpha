#pragma once

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Engine instance ----------------------------------------------------------
void *frost_engine_new();
void *frost_engine_new_with_store(const char *path);
void frost_engine_free(void *handle);
char *frost_engine_process_json(void *handle, const char *request_json);
char *frost_engine_poll_event_json(void *handle);
char *frost_engine_poll_host_command_json(void *handle);
bool frost_engine_push_host_event_json(void *handle, const char *event_json);
bool frost_engine_push_host_command_result_json(void *handle,
                                                const char *result_json);
void frost_engine_string_free(char *value);

// External / MCP automation boundary ---------------------------------------
bool frost_engine_grant_external(void *handle, const char *origin,
                                 const char *capabilities_json);
char *frost_engine_process_external_json(void *handle,
                                         const char *command_json);

// Engine-owned SQLite store (replaces native BrowserDataStore) -------------
void *frost_store_open(const char *path);
void frost_store_free(void *handle);
char *frost_store_get_setting(void *handle, const char *key);
bool frost_store_set_setting(void *handle, const char *key, const char *value);
char *frost_store_get_all_settings(void *handle);
bool frost_store_add_log(void *handle, const char *level, const char *message);
char *frost_store_get_logs(void *handle, unsigned long limit);
bool frost_store_clear_logs(void *handle);
bool frost_store_add_history(void *handle, const char *title, const char *url,
                             const char *favicon_url);
bool frost_store_upsert_download(void *handle, const char *url, const char *path,
                                 const char *state, long long percent);
// Frees a string previously returned by a frost_store_* function.
void frost_store_string_free(char *value);

#ifdef __cplusplus
}
#endif
