"use strict";

var ctx = null;

var js_objects = {};
js_objects[-1] = null;
js_objects[-2] = undefined;
var unique_js_id = 0;

function register_plugin(importObject) {
    importObject.env.js_create_string = function (buf, max_len) {
        var string = UTF8ToString(buf, max_len);
        return js_object(string);
    }

    // Copy given bytes into newly allocated Uint8Array
    importObject.env.js_create_buffer = function (buf, max_len) {
        var src = new Uint8Array(wasm_memory.buffer, buf, max_len);
        var new_buffer = new Uint8Array(new ArrayBuffer(src.byteLength));
        new_buffer.set(new Uint8Array(src));
        return js_object(new_buffer);
    }

    importObject.env.js_create_object = function () {
        var object = {};
        return js_object(object);
    }

    importObject.env.js_set_field_f32 = function (obj_id, buf, max_len, data) {
        var field = UTF8ToString(buf, max_len);

        js_objects[obj_id][field] = data;
    }

    importObject.env.js_set_field_u32 = function (obj_id, buf, max_len, data) {
        var field = UTF8ToString(buf, max_len);

        js_objects[obj_id][field] = data;
    }

    importObject.env.js_set_field_string = function (obj_id, buf, max_len, data_buf, data_len) {
        var field = UTF8ToString(buf, max_len);
        var data = UTF8ToString(data_buf, data_len);

        js_objects[obj_id][field] = data;
    }

    importObject.env.js_unwrap_to_str = function (obj_id, buf, max_len) {
        var str = js_objects[obj_id];
        var utf8array = toUTF8Array(str);
        var length = utf8array.length;
        var dest = new Uint8Array(wasm_memory.buffer, buf, max_len);
        for (var i = 0; i < length; i++) {
            dest[i] = utf8array[i];
        }
    }

    importObject.env.js_unwrap_to_buf = function (obj_id, buf, max_len) {
        var src = js_objects[obj_id];
        var length = src.length;
        var dest = new Uint8Array(wasm_memory.buffer, buf, max_len);
        for (var i = 0; i < length; i++) {
            dest[i] = src[i];
        }
    }

    importObject.env.js_string_length = function (obj_id) {
        var str = js_objects[obj_id];
        return toUTF8Array(str).length;
    }

    importObject.env.js_buf_length = function (obj_id) {
        var buf = js_objects[obj_id];
        return buf.length;
    }

    importObject.env.js_free_object = function (obj_id) {
        delete js_objects[obj_id];
    }

    importObject.env.js_have_field = function (obj_id, buf, length) {
        var field_name = UTF8ToString(buf, length);

        return js_objects[obj_id][field_name] !== undefined;
    }

    importObject.env.js_field_f32 = function (obj_id, buf, length) {
        var field_name = UTF8ToString(buf, length);

        return js_objects[obj_id][field_name];
    }

    importObject.env.js_field_u32 = function (obj_id, buf, length) {
        var field_name = UTF8ToString(buf, length);

        return js_objects[obj_id][field_name];
    }

    importObject.env.js_field = function (obj_id, buf, length) {
        var field_name = UTF8ToString(buf, length);

        var field = js_objects[obj_id][field_name];

        return js_object(field);
    }

    importObject.env.js_field_num = function (js_object, buf, length) {
        var field_name = UTF8ToString(buf, length);

        return js_objects[js_object][field_name];
    }
}
miniquad_add_plugin({ register_plugin, version: 1, name: "sapp_jsutils" });

function toUTF8Array(str) {
    var utf8 = [];
    for (var i = 0; i < str.length; i++) {
        var charcode = str.charCodeAt(i);
        if (charcode < 0x80) utf8.push(charcode);
        else if (charcode < 0x800) {
            utf8.push(0xc0 | (charcode >> 6),
                0x80 | (charcode & 0x3f));
        }
        else if (charcode < 0xd800 || charcode >= 0xe000) {
            utf8.push(0xe0 | (charcode >> 12),
                0x80 | ((charcode >> 6) & 0x3f),
                0x80 | (charcode & 0x3f));
        }
        else {
            i++;
            charcode = 0x10000 + (((charcode & 0x3ff) << 10)
                | (str.charCodeAt(i) & 0x3ff))
            utf8.push(0xf0 | (charcode >> 18),
                0x80 | ((charcode >> 12) & 0x3f),
                0x80 | ((charcode >> 6) & 0x3f),
                0x80 | (charcode & 0x3f));
        }
    }
    return utf8;
}

function js_object(obj) {
    if (obj == undefined) {
        return -2;
    }
    if (obj === null) {
        return -1;
    }
    var id = unique_js_id;

    js_objects[id] = obj;
    unique_js_id += 1;
    return id;
}

function consume_js_object(id) {
    var object = js_objects[id];
    delete js_objects[id];
    return object;
}

function get_js_object(id) {
    return js_objects[id];
}
