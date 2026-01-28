// Miniquad JS plugin for async HTTP fetch (auth API)
// Registers functions into the WASM import object so Rust can call them via extern "C"

"use strict";

const AUTH_PLUGIN = {
    requests: new Map(),
    next_id: 1,
};

miniquad_add_plugin({
    name: "auth",
    version: "0.1.0",
    register_plugin: function (importObject) {
        importObject.env.http_request = function (method_js, url_js, headers_js, body_js) {
            const method = consume_js_object(method_js);
            const url = consume_js_object(url_js);
            const headers_str = consume_js_object(headers_js);
            const body = body_js === -1 ? null : consume_js_object(body_js);

            const request_id = AUTH_PLUGIN.next_id++;
            const entry = { status: 0, response_body: null, response_status: 0 };
            AUTH_PLUGIN.requests.set(request_id, entry);

            const headers = {};
            try {
                const parsed = JSON.parse(headers_str);
                for (const key in parsed) {
                    headers[key] = parsed[key];
                }
            } catch (e) {
                console.error("auth: failed to parse headers JSON:", e);
            }

            const opts = { method: method, headers: headers };
            if (body !== null && method !== "GET" && method !== "HEAD") {
                opts.body = body;
            }

            fetch(url, opts)
                .then(function (response) {
                    entry.response_status = response.status;
                    return response.text();
                })
                .then(function (text) {
                    entry.response_body = text;
                    entry.status = 1;
                })
                .catch(function (err) {
                    console.error("auth: fetch error:", err);
                    entry.response_body = err.toString();
                    entry.status = 2;
                });

            return request_id;
        };

        importObject.env.http_poll = function (request_id) {
            const entry = AUTH_PLUGIN.requests.get(request_id);
            if (!entry) return 2;
            return entry.status;
        };

        importObject.env.http_response_body = function (request_id) {
            const entry = AUTH_PLUGIN.requests.get(request_id);
            if (!entry || entry.response_body === null) return -1;
            return js_object(entry.response_body);
        };

        importObject.env.http_response_status = function (request_id) {
            const entry = AUTH_PLUGIN.requests.get(request_id);
            if (!entry) return 0;
            return entry.response_status;
        };

        importObject.env.http_cleanup = function (request_id) {
            AUTH_PLUGIN.requests.delete(request_id);
        };
    },
});
