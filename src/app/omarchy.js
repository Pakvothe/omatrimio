// Rewrites stremio-web's own style rules where variables can't reach:
// corners, text on accents, and brand colors written by hand.
//
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

    // stremio-web still writes some of its brand's blue-violet by hand (page
    // titles, modal backgrounds, a few highlights) instead of through the
    // palette variables. Any fixed color in that hue range becomes the theme
    // color playing the same role, keeping its transparency: text takes the
    // foreground, dark surfaces the dark background, bright ones the accent.
    const TINTED = ['color', 'fill', 'background-color', 'background-image', 'border-top-color',
        'border-right-color', 'border-bottom-color', 'border-left-color', 'outline-color', 'box-shadow'];
    const RGB = /rgba?\(\s*(\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?\s*\)/g;

    const isBrandTint = (r, g, b) => {
        const max = Math.max(r, g, b), min = Math.min(r, g, b), delta = max - min;
        if (delta < 12) return false; // a gray, black, or white
        const lightness = (max + min) / 510;
        const saturation = delta / (255 - Math.abs(max + min - 255));
        let hue = max === r ? ((g - b) / delta) % 6 : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4;
        hue = (hue * 60 + 360) % 360;
        return saturation >= 0.2 && hue >= 215 && hue <= 285 ? { lightness } : false;
    };

    const retint = (style) => {
        for (const property of TINTED) {
            const value = style.getPropertyValue(property);
            if (!value || !value.includes('rgb')) continue;
            const themed = value.replace(RGB, (match, r, g, b, alpha) => {
                const tint = isBrandTint(+r, +g, +b);
                if (!tint) return match;
                const role = property === 'color' || property === 'fill'
                    ? '--primary-foreground-color'
                    : tint.lightness < 0.3 ? '--modal-background-color' : '--primary-accent-color';
                const percent = Math.round((alpha === undefined ? 1 : +alpha) * 100);
                return percent >= 100
                    ? `var(${role})`
                    : `color-mix(in srgb, var(${role}) ${percent}%, transparent)`;
            });
            if (themed !== value) style.setProperty(property, themed, style.getPropertyPriority(property));
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
            retint(rule.style);
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
