// Miniquad JS plugin for loading progress overlay
// Rust calls these to update the HTML loading bar (avoids GL buffer flashing)

"use strict";

miniquad_add_plugin({
    name: "loading",
    version: "0.1.0",
    register_plugin: function (importObject) {
        importObject.env.loading_set_progress = function (pct_times_100) {
            // pct_times_100 is 0..10000 (percent * 100 for precision)
            var pct = pct_times_100 / 100.0;
            var fill = document.getElementById("loading-bar-fill");
            var pctEl = document.getElementById("loading-pct");
            if (fill) fill.style.width = pct + "%";
            if (pctEl) pctEl.textContent = Math.floor(pct) + "%";
        };

        importObject.env.loading_set_label = function (label_js) {
            var label = consume_js_object(label_js);
            var el = document.getElementById("loading-label");
            if (el) el.textContent = label;
        };

        importObject.env.loading_hide = function () {
            var overlay = document.getElementById("loading-overlay");
            if (overlay) {
                overlay.classList.add("hidden");
                setTimeout(function () {
                    overlay.style.display = "none";
                }, 300);
            }
        };
    },
});
