// Corners follow Hyprland's window rounding, like every Omarchy app.
//
// stremio-web rounds most surfaces with var(--border-radius), which the
// injected stylesheet sets, but pills and chips use fixed lengths (3rem,
// 0.5rem...). Cap each fixed corner at the window rounding instead, so
// Omarchy's default of 0 makes them square. Circles are written as
// percentages and keep their shape. Class names carry a hash that changes
// with every stremio-web release, so this rewrites the rules themselves
// rather than targeting selectors.
(() => {
    const CORNERS = [
        'border-top-left-radius',
        'border-top-right-radius',
        'border-bottom-right-radius',
        'border-bottom-left-radius',
    ];
    const done = new WeakSet();

    const LENGTH = /^([\d.]+)(px|rem|em)$/;
    const OTHER_VARIABLE = /^var\(--(?!border-radius\))[\w-]+\)$/;

    // A fixed length or another variable (the search bar rounds by its own
    // height, var(--search-bar-size)) gets capped; percentages, zero, and
    // the theme's own var(--border-radius) are left alone.
    const shouldCap = (value) => {
        const length = value.match(LENGTH);
        return (length && parseFloat(length[1]) !== 0) || OTHER_VARIABLE.test(value);
    };

    // A square element whose radius reaches half its side is a circle (an
    // avatar is 2rem wide with a 1rem radius), and circles keep their shape.
    const isCircle = (style, value) => {
        const width = style.getPropertyValue('width').trim();
        const height = style.getPropertyValue('height').trim();
        const square = (width && width === height) || style.getPropertyValue('aspect-ratio').trim() === '1';
        if (!square) return false;
        const radius = value.match(LENGTH), side = width.match(LENGTH);
        if (!radius || !side || radius[2] !== side[2]) return true;
        return parseFloat(radius[1]) * 2 >= parseFloat(side[1]);
    };

    const cap = (style, property) => {
        const value = style.getPropertyValue(property).trim();
        if (value && shouldCap(value) && !isCircle(style, value)) {
            style.setProperty(
                property,
                `min(var(--border-radius), ${value})`,
                style.getPropertyPriority(property),
            );
        }
    };

    // Buttons painted with an accent keep stremio-web's white text, which
    // vanishes on a bright accent (Netrunner's neon green). Wherever a rule
    // paints a background with an accent, add a rule that sets the text and
    // icons inside to the color the stylesheet picked to read on it.
    const ACCENT_BACKGROUND = /var\(--((?:primary|secondary|tertiary|quaternary|warning|danger)-accent-color)\)/;

    // The last compound selector names a class like `play-icon-background-Uazjh`.
    const isBackgroundLayer = (selector) => {
        const last = selector.split(/[\s>+~]+/).pop();
        return /\.[\w-]*background[\w-]*/.test(last);
    };

    // `.a .layer .background` -> `.a .layer`
    const layerContainer = (selector) => selector.replace(/\s*[>+~]?\s*[^\s>+~]+$/, '');

    const textOnAccent = (rule, container) => {
        const background =
            rule.style.getPropertyValue('background-color') || rule.style.getPropertyValue('background');
        const accent = background.match(ACCENT_BACKGROUND);
        if (!accent) return;
        const selectors = rule.selectorText
            .split(',')
            .map((selector) => selector.trim())
            .filter((selector) => !selector.includes('::'));
        if (selectors.length === 0) return;
        const targets = selectors
            .flatMap((selector) =>
                // A background layer sits behind sibling layers (the poster's
                // play button is a background, a ring, and the icon on top,
                // with the icon first in the DOM), so everything in the
                // layer's container gets the readable color too.
                isBackgroundLayer(selector)
                    ? [selector, `${selector} *`, `${layerContainer(selector)} *`]
                    : [selector, `${selector} *`],
            )
            .join(', ');
        try {
            container.insertRule(
                `${targets} { color: var(--on-${accent[1]}) !important; }`,
                container.cssRules.length,
            );
        } catch {
            // A selector this engine can't parse; leave that rule as it was.
        }
    };

    // `container` is the sheet or grouping rule (@media, @supports) holding
    // the rules, so added rules keep their conditions.
    const capRules = (rules, container) => {
        for (const rule of Array.from(rules)) {
            if (rule.cssRules) capRules(rule.cssRules, rule);
            if (!rule.style) continue;
            // A shorthand set from a variable can't be split into corners,
            // so the browser reports it only on the shorthand.
            cap(rule.style, 'border-radius');
            for (const corner of CORNERS) cap(rule.style, corner);
            if (rule.selectorText) textOnAccent(rule, container);
        }
    };

    const scan = () => {
        for (const sheet of document.styleSheets) {
            if (done.has(sheet)) continue;
            try {
                capRules(sheet.cssRules, sheet);
                done.add(sheet);
                console.debug(`omatrimio: restyled ${sheet.href || 'inline style'} (${sheet.cssRules.length} rules)`);
            } catch (error) {
                console.debug(`omatrimio: skipped ${sheet.href}: ${error}`);
                // A stylesheet from another origin can't be read; leave it be.
            }
        }
    };

    // Stylesheets arrive as <link> loads (captured on the document) and as
    // <style> elements that bundles add at runtime.
    document.addEventListener('load', scan, true);
    document.addEventListener('DOMContentLoaded', () => {
        scan();
        new MutationObserver(scan).observe(document.head, { childList: true });
    });
})();
