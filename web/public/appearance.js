      (function () {
        var html = document.documentElement;

        var themePref = localStorage.getItem("lific_theme");
        var dark =
          themePref === "dark" ||
          (!themePref && window.matchMedia("(prefers-color-scheme: dark)").matches);
        if (dark) html.classList.add("dark");

        var accent = localStorage.getItem("lific_accent");
        html.setAttribute(
          "data-accent",
          ["teal", "rose", "amber", "green", "violet"].indexOf(accent) !== -1
            ? accent
            : "indigo",
        );

        if (localStorage.getItem("lific_density") === "compact") {
          html.classList.add("density-compact");
        }

        var fontScale = localStorage.getItem("lific_font_scale");
        html.setAttribute(
          "data-font-scale",
          fontScale === "sm" || fontScale === "lg" ? fontScale : "md",
        );

        var motionPref = localStorage.getItem("lific_motion");
        var reducedMotion =
          motionPref === "reduced" ||
          (motionPref !== "full" &&
            window.matchMedia("(prefers-reduced-motion: reduce)").matches);
        html.setAttribute("data-motion", reducedMotion ? "reduced" : "full");
      })();
