// Miniquad JS plugin for WebSocket networking
// Registers functions into the WASM import object so Rust can call them via extern "C"

"use strict";

const WS_PLUGIN = {
    ws: null,
    recv_queue: [],
    has_opened: false,
    has_closed: false,
    has_error: false,
};

miniquad_add_plugin({
    name: "network",
    version: "0.1.0",
    register_plugin: function (importObject) {
        importObject.env.ws_connect = function (url_js_obj) {
            const url = consume_js_object(url_js_obj);
            try {
                WS_PLUGIN.ws = new WebSocket(url);
                WS_PLUGIN.ws.binaryType = "arraybuffer";
                WS_PLUGIN.has_opened = false;
                WS_PLUGIN.has_closed = false;
                WS_PLUGIN.has_error = false;
                WS_PLUGIN.recv_queue = [];

                WS_PLUGIN.ws.onopen = function () {
                    WS_PLUGIN.has_opened = true;
                };
                WS_PLUGIN.ws.onclose = function () {
                    WS_PLUGIN.has_closed = true;
                };
                WS_PLUGIN.ws.onerror = function () {
                    WS_PLUGIN.has_error = true;
                };
                WS_PLUGIN.ws.onmessage = function (event) {
                    if (event.data instanceof ArrayBuffer) {
                        WS_PLUGIN.recv_queue.push(new Uint8Array(event.data));
                    }
                };
            } catch (e) {
                console.error("ws_connect failed:", e);
                WS_PLUGIN.has_error = true;
            }
        };

        importObject.env.ws_disconnect = function () {
            if (WS_PLUGIN.ws != null) {
                try { WS_PLUGIN.ws.close(); } catch (_) {}
                WS_PLUGIN.ws = null;
            }
        };

        importObject.env.ws_send = function (data_js_obj) {
            if (WS_PLUGIN.ws != null && WS_PLUGIN.ws.readyState === WebSocket.OPEN) {
                const data = consume_js_object(data_js_obj);
                WS_PLUGIN.ws.send(data);
            }
        };

        importObject.env.ws_try_recv = function () {
            if (WS_PLUGIN.recv_queue.length === 0) {
                return -1;
            }
            const data = WS_PLUGIN.recv_queue.shift();
            return js_object(data);
        };

        importObject.env.ws_is_connected = function () {
            return (WS_PLUGIN.ws != null && WS_PLUGIN.ws.readyState === WebSocket.OPEN) ? 1 : 0;
        };

        importObject.env.ws_has_error = function () {
            if (WS_PLUGIN.has_error) {
                WS_PLUGIN.has_error = false;
                return 1;
            }
            return 0;
        };

        importObject.env.ws_has_closed = function () {
            if (WS_PLUGIN.has_closed) {
                WS_PLUGIN.has_closed = false;
                return 1;
            }
            return 0;
        };

        importObject.env.ws_has_opened = function () {
            if (WS_PLUGIN.has_opened) {
                WS_PLUGIN.has_opened = false;
                return 1;
            }
            return 0;
        };
    },
});
