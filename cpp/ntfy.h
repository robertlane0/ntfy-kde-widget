/*
    SPDX-License-Identifier: MIT
*/
#pragma once

#include <cstddef>
#include <cstdint>

// Mirror of the Rust core's C ABI.
extern "C" {

/// Event kinds delivered to a sink.
enum NtfyEventKind : uint32_t {
    NtfyEventState = 1,
    NtfyEventNotify = 2,
    NtfyEventLog = 3,
};

/// Commands accepted by ntfy_command.
enum NtfyCommand : uint32_t {
    NtfyCmdAdd = 1,
    NtfyCmdRemove = 2,
    NtfyCmdSetEnabled = 3,
    NtfyCmdMarkRead = 4,
    NtfyCmdMarkAllRead = 5,
};

/// Starts the engine and every subscription stream. Returns non-zero on success.
int32_t ntfy_start();

/// Stops all streams.
void ntfy_stop();

bool ntfy_is_running();

/// Runs a command and returns a JSON result the caller frees with ntfy_string_free.
/// Any argument pointer may be null, meaning an empty string.
char *ntfy_command(uint32_t op, const char *a, const char *b, const char *c);

void ntfy_string_free(char *s);

/// Registers a listener. Returns an opaque handle for ntfy_remove_sink.
size_t ntfy_add_sink(void (*sink)(uint32_t kind, const char *json, size_t len));

void ntfy_remove_sink(size_t handle);

}