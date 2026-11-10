// Stremio's own artwork (its logo, the cat avatar, the empty-state
// illustrations) is swapped for omatrimio's. Each image is tagged with what
// it shows, and the injected stylesheet draws omatrimio's art for that tag
// in the theme's colors, so it follows theme switches like everything else.
//
// Images arrive as <img src> and as inline background-image styles (avatars),
// both under hashed build paths, so they're matched by file name.
(() => {
    const ART = 'data-omatrimio-art';
    const KINDS = [
        [/\/images\/logo\.png/, 'wordmark'],
        [/\/images\/stremio_symbol\.png/, 'logo'],
        [/\/images\/(default_avatar|anonymous|avatar_fallback)\.png|stremio\.com\/images\/avatars\//, 'avatar'],
        [/\/images\/(empty|library_placeholder|calendar_placeholder)\.png/, 'empty'],
    ];

    const kindOf = (url) => KINDS.find(([pattern]) => pattern.test(url))?.[1];

    const tag = (element) => {
        const url = element.localName === 'img'
            ? element.getAttribute('src')
            : element.style?.backgroundImage;
        const kind = url ? kindOf(url) : undefined;
        if (kind) {
            if (element.getAttribute(ART) !== kind) element.setAttribute(ART, kind);
        } else if (element.hasAttribute(ART)) {
            element.removeAttribute(ART);
        }
    };

    const scan = (node) => {
        if (node.nodeType !== Node.ELEMENT_NODE) return;
        tag(node);
        node.querySelectorAll('img, [style*="background-image"]').forEach(tag);
    };

    const observer = new MutationObserver((records) => {
        for (const record of records) {
            if (record.type === 'attributes') tag(record.target);
            else record.addedNodes.forEach(scan);
        }
    });

    document.addEventListener('DOMContentLoaded', () => {
        scan(document.body);
        observer.observe(document.body, {
            childList: true,
            subtree: true,
            attributes: true,
            attributeFilter: ['src', 'style'],
        });
    });
})();
