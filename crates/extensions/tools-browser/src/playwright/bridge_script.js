
const { chromium } = require('playwright');
const readline = require('readline');

// State
const browsers = new Map();
const pages = new Map();
let idCounter = 1;

// JSON-RPC handler
async function handleRequest(request) {
    const { id, method, params } = request;

    try {
        let result;
        switch (method) {
            case 'ping':
                result = 'pong';
                break;

            case 'shutdown':
                // Close all browsers
                for (const browser of browsers.values()) {
                    await browser.close().catch(() => {});
                }
                process.exit(0);
                break;

            case 'launchBrowser':
                result = await launchBrowser(params);
                break;

            case 'connectBrowser':
                result = await connectBrowser(params);
                break;

            case 'closeBrowser':
                await closeBrowser(params);
                result = null;
                break;

            case 'newPage':
                result = await newPage(params);
                break;

            case 'navigate':
                await navigate(params);
                result = null;
                break;

            case 'click':
                await click(params);
                result = null;
                break;

            case 'clickSelector':
                await clickSelector(params);
                result = null;
                break;

            case 'typeText':
                await typeText(params);
                result = null;
                break;

            case 'fill':
                await fill(params);
                result = null;
                break;

            case 'pressKey':
                await pressKey(params);
                result = null;
                break;

            case 'screenshot':
                result = await screenshot(params);
                break;

            case 'getContent':
                result = await getContent(params);
                break;

            case 'getUrl':
                result = await getUrl(params);
                break;

            case 'getTitle':
                result = await getTitle(params);
                break;

            case 'evaluate':
                result = await evaluate(params);
                break;

            case 'waitForSelector':
                await waitForSelector(params);
                result = null;
                break;

            case 'closePage':
                await closePage(params);
                result = null;
                break;

            case 'goBack':
                await goBack(params);
                result = null;
                break;

            case 'goForward':
                await goForward(params);
                result = null;
                break;

            case 'reload':
                await reload(params);
                result = null;
                break;

            case 'scroll':
                await scroll(params);
                result = null;
                break;

            case 'getDomTree':
                result = await getDomTree(params);
                break;

            case 'elementAt':
                result = await elementAt(params);
                break;

            default:
                throw new Error(`Unknown method: ${method}`);
        }

        return { id, result };
    } catch (error) {
        return {
            id,
            error: {
                message: error.message,
                code: -1
            }
        };
    }
}

// Browser methods
async function launchBrowser({ headless = true, args = [] }) {
    const browser = await chromium.launch({
        headless,
        args: [
            '--disable-blink-features=AutomationControlled',
            '--disable-gpu',
            '--no-sandbox',
            ...args
        ]
    });

    const browserId = `browser_${idCounter++}`;
    browsers.set(browserId, browser);
    return browserId;
}

async function connectBrowser({ endpoint }) {
    const browser = await chromium.connectOverCDP(endpoint);
    const browserId = `browser_${idCounter++}`;
    browsers.set(browserId, browser);
    return browserId;
}

async function closeBrowser({ browserId }) {
    const browser = browsers.get(browserId);
    if (browser) {
        await browser.close();
        browsers.delete(browserId);
    }
}

// Page methods
async function newPage({ browserId }) {
    const browser = browsers.get(browserId);
    if (!browser) throw new Error(`Browser not found: ${browserId}`);

    const context = await browser.newContext({
        viewport: { width: 1280, height: 720 }
    });
    const page = await context.newPage();

    const pageId = `page_${idCounter++}`;
    pages.set(pageId, { page, context, browserId });
    return pageId;
}

async function navigate({ pageId, url, waitUntil = 'domcontentloaded' }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.goto(url, { waitUntil });
}

async function click({ pageId, x, y }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.mouse.click(x, y);
}

async function clickSelector({ pageId, selector }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.click(selector);
}

async function typeText({ pageId, text, delay = 0 }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.keyboard.type(text, { delay });
}

async function fill({ pageId, selector, value }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.fill(selector, value);
}

async function pressKey({ pageId, key }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.keyboard.press(key);
}

async function screenshot({ pageId, options = {} }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    const buffer = await pageData.page.screenshot({
        type: options.type || 'png',
        fullPage: options.fullPage || false,
        quality: options.quality,
        clip: options.clip
    });

    return buffer.toString('base64');
}

async function getContent({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.content();
}

async function getUrl({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return pageData.page.url();
}

async function getTitle({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.title();
}

async function evaluate({ pageId, script }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.evaluate(script);
}

async function waitForSelector({ pageId, selector, timeout = 30000 }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.waitForSelector(selector, { timeout });
}

async function closePage({ pageId }) {
    const pageData = pages.get(pageId);
    if (pageData) {
        await pageData.page.close();
        await pageData.context.close();
        pages.delete(pageId);
    }
}

async function goBack({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.goBack();
}

async function goForward({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.goForward();
}

async function reload({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.reload();
}

async function scroll({ pageId, x, y }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    await pageData.page.evaluate(({ x, y }) => {
        window.scrollBy(x, y);
    }, { x, y });
}

// DOM Analysis (Browser-Use Style)
async function getDomTree({ pageId }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    const page = pageData.page;

    // Get viewport info
    const viewport = page.viewportSize() || { width: 1280, height: 720 };
    const scrollInfo = await page.evaluate(() => ({
        x: window.scrollX,
        y: window.scrollY,
        devicePixelRatio: window.devicePixelRatio
    }));

    // Get all interactive elements with their properties
    const nodes = await page.evaluate(() => {
        const results = [];
        let nodeId = 0;

        // Interactive selectors
        const interactiveSelectors = [
            'a', 'button', 'input', 'select', 'textarea', 'option',
            '[role="button"]', '[role="link"]', '[role="checkbox"]',
            '[role="radio"]', '[role="menuitem"]', '[role="tab"]',
            '[onclick]', '[tabindex]', '[contenteditable="true"]'
        ];

        // 10-layer clickable detection
        function calculateClickability(el) {
            let score = 0;
            const reasons = [];

            const tag = el.tagName.toLowerCase();
            const style = window.getComputedStyle(el);
            const role = el.getAttribute('role');

            // Layer 1: Native interactive tags
            const interactiveTags = ['a', 'button', 'input', 'select', 'textarea', 'option', 'label'];
            if (interactiveTags.includes(tag)) {
                score += 0.3;
                reasons.push(`native_tag:${tag}`);
            }

            // Layer 2: ARIA roles
            const clickableRoles = ['button', 'link', 'checkbox', 'radio', 'menuitem', 'tab', 'option', 'switch'];
            if (role && clickableRoles.includes(role)) {
                score += 0.2;
                reasons.push(`aria_role:${role}`);
            }

            // Layer 3: Cursor pointer
            if (style.cursor === 'pointer') {
                score += 0.15;
                reasons.push('cursor_pointer');
            }

            // Layer 4: Has href
            if (el.hasAttribute('href')) {
                score += 0.2;
                reasons.push('has_href');
            }

            // Layer 5: Event handlers
            const eventAttrs = ['onclick', 'onmousedown', 'onmouseup', 'ontouchstart'];
            for (const attr of eventAttrs) {
                if (el.hasAttribute(attr)) {
                    score += 0.15;
                    reasons.push(`event:${attr}`);
                    break;
                }
            }

            // Layer 6: Tabindex
            const tabindex = el.getAttribute('tabindex');
            if (tabindex !== null && parseInt(tabindex) >= 0) {
                score += 0.1;
                reasons.push('tabindex');
            }

            // Layer 7: Input type
            if (tag === 'input') {
                const type = el.getAttribute('type') || 'text';
                const clickableTypes = ['button', 'submit', 'reset', 'checkbox', 'radio', 'file'];
                if (clickableTypes.includes(type)) {
                    score += 0.15;
                    reasons.push(`input_type:${type}`);
                }
            }

            // Layer 8: Contenteditable
            if (el.isContentEditable) {
                score += 0.2;
                reasons.push('contenteditable');
            }

            return { score: Math.min(score, 1.0), reasons };
        }

        // Get element info
        function getElementInfo(el) {
            const rect = el.getBoundingClientRect();
            const style = window.getComputedStyle(el);

            // Check visibility
            const isVisible = rect.width > 0 &&
                rect.height > 0 &&
                style.visibility !== 'hidden' &&
                style.display !== 'none' &&
                style.opacity !== '0';

            // Check if in viewport
            const isInViewport = rect.top < window.innerHeight &&
                rect.bottom > 0 &&
                rect.left < window.innerWidth &&
                rect.right > 0;

            const { score, reasons } = calculateClickability(el);

            // Get text content (direct text only)
            let textContent = '';
            for (const node of el.childNodes) {
                if (node.nodeType === Node.TEXT_NODE) {
                    textContent += node.textContent;
                }
            }
            textContent = textContent.trim().substring(0, 200);

            // Get attributes
            const attrs = {};
            for (const attr of el.attributes) {
                if (attr.name.startsWith('data-')) {
                    if (!attrs.data) attrs.data = {};
                    attrs.data[attr.name.substring(5)] = attr.value;
                } else {
                    const attrMap = {
                        'id': 'id', 'class': 'class', 'href': 'href', 'src': 'src',
                        'alt': 'alt', 'title': 'title', 'placeholder': 'placeholder',
                        'value': 'value', 'type': 'type', 'name': 'name', 'role': 'role',
                        'aria-label': 'aria_label', 'aria-expanded': 'aria_expanded',
                        'aria-selected': 'aria_selected'
                    };
                    if (attrMap[attr.name]) {
                        attrs[attrMap[attr.name]] = attr.value;
                    }
                }
            }

            // Generate unique selector
            let cssSelector = el.tagName.toLowerCase();
            if (el.id) {
                cssSelector = `#${el.id}`;
            } else if (el.className) {
                cssSelector += '.' + el.className.split(' ').filter(c => c).join('.');
            }

            // Generate XPath
            function getXPath(element) {
                if (element.id) return `//*[@id="${element.id}"]`;

                const parts = [];
                let current = element;
                while (current && current.nodeType === Node.ELEMENT_NODE) {
                    let index = 0;
                    let sibling = current.previousSibling;
                    while (sibling) {
                        if (sibling.nodeType === Node.ELEMENT_NODE &&
                            sibling.tagName === current.tagName) {
                            index++;
                        }
                        sibling = sibling.previousSibling;
                    }
                    const tag = current.tagName.toLowerCase();
                    parts.unshift(index > 0 ? `${tag}[${index + 1}]` : tag);
                    current = current.parentElement;
                }
                return '/' + parts.join('/');
            }

            return {
                id: `node_${nodeId++}`,
                backend_node_id: nodeId,
                tag_name: el.tagName.toLowerCase(),
                attributes: attrs,
                text_content: textContent,
                bounding_box: {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height
                },
                is_visible: isVisible,
                is_in_viewport: isInViewport,
                clickability_score: score,
                clickability_reasons: reasons,
                paint_order: 0, // Would need layerTree for accurate z-index
                is_interactive: score > 0.3,
                is_focusable: el.tabIndex >= 0,
                parent_id: null,
                children: [],
                xpath: getXPath(el),
                css_selector: cssSelector,
                computed_styles: {
                    cursor: style.cursor,
                    display: style.display,
                    visibility: style.visibility
                }
            };
        }

        // Collect all potentially interactive elements
        const elements = document.querySelectorAll(interactiveSelectors.join(','));
        for (const el of elements) {
            try {
                const info = getElementInfo(el);
                if (info.is_visible) {
                    results.push(info);
                }
            } catch (e) {
                // Skip problematic elements
            }
        }

        return results;
    });

    // Build node map
    const nodeMap = {};
    for (const node of nodes) {
        nodeMap[node.id] = node;
    }

    return {
        roots: nodes.filter(n => !n.parent_id).map(n => n.id),
        nodes: nodeMap,
        viewport: {
            width: viewport.width,
            height: viewport.height,
            device_pixel_ratio: scrollInfo.devicePixelRatio || 1,
            scroll_x: scrollInfo.x || 0,
            scroll_y: scrollInfo.y || 0
        },
        timestamp: Date.now(),
        url: page.url(),
        title: await page.title()
    };
}

async function elementAt({ pageId, x, y }) {
    const pageData = pages.get(pageId);
    if (!pageData) throw new Error(`Page not found: ${pageId}`);

    return await pageData.page.evaluate(({ x, y }) => {
        const el = document.elementFromPoint(x, y);
        if (!el) return null;

        return {
            tagName: el.tagName.toLowerCase(),
            id: el.id || null,
            className: el.className || null,
            textContent: el.textContent?.substring(0, 100) || null
        };
    }, { x, y });
}

// Main loop
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false
});

rl.on('line', async (line) => {
    try {
        const request = JSON.parse(line);
        const response = await handleRequest(request);
        console.log(JSON.stringify(response));
    } catch (error) {
        console.log(JSON.stringify({
            id: 0,
            error: { message: error.message, code: -1 }
        }));
    }
});

// Handle uncaught errors
process.on('uncaughtException', (error) => {
    console.error('Uncaught exception:', error);
});

process.on('unhandledRejection', (error) => {
    console.error('Unhandled rejection:', error);
});

console.error('Playwright bridge started');
