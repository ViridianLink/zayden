const CHECK_URL = "/admin/destiny2/loadouts/editor/check";
const SAVE_URL = "/admin/destiny2/loadouts/editor/save";
const EMOJI_URL = "/admin/destiny2/loadouts/editor/emoji";
const CHECK_DELAY = 400;
const MAX_IMAGE_BYTES = 256 * 1024;
const MAX = {
    aspects: 2,
    fragments: 6,
    weapons: 3,
    perks: 5,
    mods: 5,
    artifactPerks: 12,
    tags: 3,
};
const REORDER_HINT = "Drag, or press Alt + arrow keys, to reorder";
const EMOJI_CDN = "https://cdn.discordapp.com/emojis/";
const SMALL_WORDS = ["a", "an", "and", "at", "for", "in", "of", "on", "the"];

const ICON_OPEN =
    '<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">';
const ICON = {
    plus: ICON_OPEN + '<path d="M5 12h14"/><path d="M12 5v14"/></svg>',
    x: ICON_OPEN + '<path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>',
    grip:
        ICON_OPEN +
        '<circle cx="9" cy="6" r="1"/><circle cx="15" cy="6" r="1"/><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/><circle cx="9" cy="18" r="1"/><circle cx="15" cy="18" r="1"/></svg>',
};

const FIELDS = {
    super: { usage: "super", noun: "super", emoji: true },
    classAbility: {
        usage: "class_ability",
        noun: "class ability",
        emoji: true,
    },
    jump: { usage: "jump", noun: "jump", emoji: true },
    melee: { usage: "melee", noun: "melee", emoji: true },
    grenade: { usage: "grenade", noun: "grenade", emoji: true },
    aspect: { usage: "aspect", noun: "aspect", emoji: true },
    fragment: { usage: "fragment", noun: "fragment", emoji: true },
    weaponPerk: { usage: "weapon_perk", noun: "perk", emoji: true },
    armourMod: { usage: "armour_mod", noun: "mod", emoji: true, repeats: true },
    artifactPerk: {
        usage: "artifact_perk",
        noun: "artifact perk",
        emoji: true,
    },
    weapon: { usage: "weapon", noun: "weapon" },
    armour: { usage: "armour", noun: "armour piece" },
    artifact: { usage: "artifact", noun: "artifact" },
};

// Unicode White_Space, as the server trims: unlike String.prototype.trim it
// keeps U+FEFF and removes U+0085.
const WS =
    "[\\t\\n\\v\\f\\r \\u0085\\u00a0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000]";
const TRIM = new RegExp("^" + WS + "+|" + WS + "+$", "gu");
const SPACE = new RegExp("^" + WS + "$", "u");
const SPACES = new RegExp(WS + "+", "u");

function trim(value) {
    return value.replace(TRIM, "");
}

const ESCAPES = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
};

function esc(value) {
    return String(value).replace(/[&<>"']/g, (c) => ESCAPES[c]);
}

function displayName(key) {
    return key
        .split("_")
        .filter((w) => w !== "")
        .map((word, i) => {
            if (i > 0 && SMALL_WORDS.includes(word)) return word;
            const [first, ...rest] = [...word];
            return first.toUpperCase() + rest.join("");
        })
        .join(" ");
}

function keyFromQuery(query) {
    let key = "";
    for (const c of trim(query)) {
        if (/^[A-Za-z0-9]$/.test(c)) key += c.toLowerCase();
        else if (
            (SPACE.test(c) || c === "_" || c === "-") &&
            !key.endsWith("_")
        )
            key += "_";
    }
    key = key.replace(/^_+|_+$/g, "");
    return [...key].slice(0, 32).join("").replace(/_+$/, "");
}

function isValidKey(key) {
    return key.length >= 2 && key.length <= 32 && /^[a-z0-9_]+$/.test(key);
}

function enumKey(label) {
    return trim(label).toLowerCase().replaceAll(" ", "_");
}

function httpsPreview(url) {
    const u = trim(url);
    return u.startsWith("https://") ? u : null;
}

const isStr = (v) => typeof v === "string";
const isStrs = (v) => Array.isArray(v) && v.every(isStr);
const isRows = (v, check) =>
    Array.isArray(v) &&
    v.every((r) => r !== null && typeof r === "object" && check(r));

function canon(f) {
    return {
        id: f.id,
        name: f.name,
        class: f.class,
        element: f.element,
        mode: f.mode,
        tags: [...f.tags],
        super_name: f.super_name,
        super_emoji: f.super_emoji,
        class_ability: f.class_ability,
        jump: f.jump,
        melee: f.melee,
        grenade: f.grenade,
        aspects: f.aspects.map((a) => ({
            aspect: a.aspect,
            fragments: [...a.fragments],
        })),
        weapons: f.weapons.map((w) => ({
            name: w.name,
            affinity: w.affinity,
            archetype: w.archetype,
            icon_url: w.icon_url,
            perks: [...w.perks],
        })),
        armour: f.armour.map((a) => ({
            slot: a.slot,
            name: a.name,
            icon_url: a.icon_url,
            mods: [...a.mods],
        })),
        stats: f.stats.map((s) => ({ stat: s.stat, value: s.value })),
        artifact_name: f.artifact_name,
        artifact_perks: [...f.artifact_perks],
        author: f.author,
        dim_link: f.dim_link,
        video_url: f.video_url,
        how_it_works: f.how_it_works,
    };
}

function validForm(f) {
    if (f === null || typeof f !== "object") return false;
    const strings = [
        "name",
        "class",
        "element",
        "mode",
        "super_name",
        "super_emoji",
        "class_ability",
        "jump",
        "melee",
        "grenade",
        "artifact_name",
        "author",
        "dim_link",
        "video_url",
        "how_it_works",
    ];
    return (
        (f.id === null || Number.isInteger(f.id)) &&
        strings.every((k) => isStr(f[k])) &&
        isStrs(f.tags) &&
        isStrs(f.artifact_perks) &&
        isRows(f.aspects, (a) => isStr(a.aspect) && isStrs(a.fragments)) &&
        isRows(
            f.weapons,
            (w) =>
                isStr(w.name) &&
                isStr(w.affinity) &&
                isStr(w.archetype) &&
                isStr(w.icon_url) &&
                isStrs(w.perks),
        ) &&
        isRows(
            f.armour,
            (a) =>
                isStr(a.slot) &&
                isStr(a.name) &&
                isStr(a.icon_url) &&
                isStrs(a.mods),
        ) &&
        isRows(f.stats, (s) => isStr(s.stat) && isStr(s.value))
    );
}

const same = (a, b) => JSON.stringify(canon(a)) === JSON.stringify(canon(b));

function emojiKeys(f) {
    const keys = [f.super_emoji, f.class_ability, f.jump, f.melee, f.grenade];
    for (const a of f.aspects) keys.push(a.aspect, ...a.fragments);
    for (const w of f.weapons) keys.push(...w.perks);
    for (const a of f.armour) keys.push(...a.mods);
    keys.push(...f.artifact_perks);
    return keys.filter((k) => trim(k) !== "");
}

function moveItem(items, from, to) {
    if (from !== to && from < items.length && to < items.length) {
        const [item] = items.splice(from, 1);
        items.splice(to, 0, item);
    }
}

function buildIndex(c) {
    const o = c.options;
    const reserved = new Set(
        [
            o.classes,
            o.elements,
            o.modes,
            o.affinities,
            o.archetypes,
            o.stats,
            o.armour_slots,
        ]
            .flat()
            .map(enumKey),
    );
    const images = new Map(
        c.emojis.map((e) => [e.name, EMOJI_CDN + e.id + ".webp?size=64"]),
    );
    return {
        images,
        reserved,
        image: (key) => images.get(key) ?? null,
        hasEmoji: (key) => images.has(key),
        hasAny: () => images.size > 0,
        enumImage: (label) => images.get(enumKey(label)) ?? null,
    };
}

function matches(query, label, key) {
    const haystack =
        label.toLowerCase() + " " + key.toLowerCase().replaceAll("_", " ");
    return query
        .toLowerCase()
        .split(SPACES)
        .filter((w) => w !== "")
        .every((word) => haystack.includes(word));
}

const byKey = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

function usedKeys(c, scope) {
    const here = [];
    const elsewhere = new Map();
    for (const u of c.usage) {
        if (u.field !== FIELDS[scope.field].usage) continue;
        if (u.class === scope.class && u.element === scope.element)
            here.push([u.uses, u.key]);
        else elsewhere.set(u.key, (elsewhere.get(u.key) ?? 0) + u.uses);
    }
    const order = (a, b) => b[0] - a[0] || byKey(a[1], b[1]);
    const rest = [...elsewhere].map(([k, n]) => [n, k]);
    here.sort(order);
    rest.sort(order);
    return [here.map((x) => x[1]), rest.map((x) => x[1])];
}

function sections(c, idx, scope, query) {
    const seen = new Set();
    const out = [];
    const push = (title, items) => {
        const kept = [];
        for (const item of items) {
            if (!matches(query, item.label, item.key)) continue;
            if (seen.has(item.key)) continue;
            seen.add(item.key);
            kept.push({ ...item, selected: scope.taken.includes(item.key) });
        }
        if (kept.length > 0) out.push({ title, items: kept });
    };
    const emoji = (key) => ({
        key,
        label: displayName(key),
        detail: key,
        image: idx.image(key),
    });
    const weapon = (w) => ({
        key: w.name,
        label: w.name,
        detail: w.affinity + " " + w.archetype,
        image: w.icon_url !== "" ? w.icon_url : null,
    });
    const hereTitle =
        "Used in " + scope.element + " " + scope.class + " builds";

    if (scope.field === "weapon") {
        const [here] = usedKeys(c, scope);
        push(
            hereTitle,
            here
                .map((name) => c.weapons.find((w) => w.name === name))
                .filter(Boolean)
                .map(weapon),
        );
        push("All weapons", c.weapons.map(weapon));
    } else if (scope.field === "artifact") {
        const [here, elsewhere] = usedKeys(c, scope);
        const named = (name) => ({
            key: name,
            label: name,
            detail: "",
            image: null,
        });
        push(hereTitle, here.map(named));
        push("Used elsewhere", elsewhere.map(named));
    } else if (scope.field === "armour") {
        const slot = scope.armourSlot ?? "";
        const pieces = c.armour.filter((a) => a.slot === slot);
        const piece = (a) => ({
            key: a.name,
            label: a.name,
            detail: a.class + " " + a.slot,
            image: a.icon_url !== "" ? a.icon_url : null,
        });
        push(
            "Used in " + scope.class + " builds",
            pieces.filter((a) => a.class === scope.class).map(piece),
        );
        push(
            "Other classes",
            pieces.filter((a) => a.class !== scope.class).map(piece),
        );
    } else {
        if (scope.field === "weaponPerk" && scope.weapon) {
            const w = c.weapons.find((x) => x.name === scope.weapon);
            if (w)
                push(
                    "Known perks for " + scope.weapon,
                    w.known_perks.map(emoji),
                );
        }
        const [here, elsewhere] = usedKeys(c, scope);
        push(hereTitle, here.map(emoji));
        push("Used elsewhere", elsewhere.map(emoji));
        const other = [...idx.images.keys()];
        if (scope.field === "weaponPerk") other.push(...c.perks);
        const sorted = [...new Set(other)].sort(byKey);
        push(
            "Other Zayden emojis",
            sorted.filter((k) => !idx.reserved.has(k)).map(emoji),
        );
    }
    return out;
}

function storage() {
    try {
        return window.sessionStorage;
    } catch {
        return null;
    }
}

function loadDraft(key) {
    try {
        const raw = storage()?.getItem(key);
        if (raw == null) return null;
        const draft = JSON.parse(raw);
        return validForm(draft) ? canon(draft) : null;
    } catch {
        return null;
    }
}

function storeDraft(key, f) {
    try {
        storage()?.setItem(key, JSON.stringify(canon(f)));
    } catch {
        // storage full or blocked: the draft is best-effort
    }
}

function clearDraft(key) {
    try {
        storage()?.removeItem(key);
    } catch {
        // blocked storage has nothing to clear
    }
}

let handlerList = [];
const handlers = new WeakMap();
let changes = 0;
let diffs = [];

function on(handler) {
    handlerList.push(handler);
    return ' data-h="' + (handlerList.length - 1) + '"';
}

function parse(html) {
    const t = document.createElement("template");
    t.innerHTML = html;
    for (const el of t.content.querySelectorAll("[data-h]")) {
        handlers.set(el, handlerList[Number(el.getAttribute("data-h"))]);
        el.removeAttribute("data-h");
    }
    return t.content;
}

function note(what, node) {
    changes += 1;
    if (diffs.length < 20) {
        const where =
            node.nodeType === 1
                ? node.nodeName.toLowerCase() +
                  (node.id ? "#" + node.id : "") +
                  "." +
                  (node.getAttribute("class") ?? "")
                : node.nodeName;
        diffs.push(what + " @ " + where);
    }
}

function nodeKey(n) {
    if (n.nodeType !== 1) return n.nodeName;
    const id = n.getAttribute("id");
    if (id) return n.nodeName + "#" + id;
    return n.nodeName + "." + (n.getAttribute("class") ?? "").split(" ")[0];
}

function morphAttributes(o, n) {
    for (const { name, value } of [...n.attributes]) {
        if (o.getAttribute(name) !== value) {
            o.setAttribute(name, value);
            note("attr " + name + "=" + value, o);
        }
    }
    for (const { name } of [...o.attributes]) {
        if (!n.hasAttribute(name)) {
            o.removeAttribute(name);
            note("remove attr " + name, o);
        }
    }
}

function morph(o, n) {
    if (o.nodeType === 3) {
        if (o.nodeValue !== n.nodeValue) {
            o.nodeValue = n.nodeValue;
            note("text " + n.nodeValue, o.parentNode);
        }
        return;
    }
    morphAttributes(o, n);
    if (handlers.has(n)) handlers.set(o, handlers.get(n));
    else handlers.delete(o);
    if (o.nodeName === "TEXTAREA") {
        if (o.value !== n.value) {
            o.value = n.value;
            note("value", o);
        }
        return;
    }
    morphChildren(o, n);
    if (o.nodeName === "INPUT" && (o.type === "radio" || o.type === "checkbox")) {
        const checked = n.hasAttribute("checked");
        if (o.checked !== checked) {
            o.checked = checked;
            note("checked", o);
        }
    }
    if (o.nodeName === "INPUT" && o.type !== "file") {
        const value = n.getAttribute("value") ?? "";
        if (o.value !== value) {
            o.value = value;
            note("value", o);
        }
    }
    if (o.nodeName === "SELECT" && o.value !== n.value) {
        o.value = n.value;
        note("value", o);
    }
}

function morphChildren(oldParent, newParent) {
    for (const child of [...oldParent.childNodes]) {
        if (child.nodeType === 8) child.remove();
    }
    const fresh = [...newParent.childNodes];
    let i = 0;
    for (const n of fresh) {
        const key = nodeKey(n);
        const o = oldParent.childNodes[i];
        if (o && nodeKey(o) === key) {
            morph(o, n);
            i += 1;
            continue;
        }
        let match = null;
        for (let j = i + 1; j < oldParent.childNodes.length; j += 1) {
            if (nodeKey(oldParent.childNodes[j]) === key) {
                match = oldParent.childNodes[j];
                break;
            }
        }
        if (match) {
            while (oldParent.childNodes[i] !== match) {
                note("remove", oldParent.childNodes[i]);
                oldParent.childNodes[i].remove();
            }
            morph(match, n);
        } else {
            oldParent.insertBefore(n, oldParent.childNodes[i] ?? null);
            note("insert", n);
        }
        i += 1;
    }
    while (oldParent.childNodes.length > i) {
        note("remove", oldParent.lastChild);
        oldParent.lastChild.remove();
    }
}

export const helpers = {
    displayName,
    keyFromQuery,
    isValidKey,
    enumKey,
    matches,
    sections,
    buildIndex,
    canon,
    same,
    validForm,
    moveItem,
    emojiKeys,
    fields: FIELDS,
};

const dataNode =
    typeof document === "undefined"
        ? null
        : document.getElementById("loadout-editor-data");
const form = dataNode && document.querySelector("form.loadout-editor");
const dialog = dataNode && document.querySelector("dialog.picker");

if (dataNode && form && dialog) start(JSON.parse(dataNode.textContent));

function start(data) {
    const catalog = data.catalog;
    const draftKey = data.draftKey;
    let index = buildIndex(catalog);
    let saved = canon(data.form);
    let s = canon(data.form);

    const ui = {
        restored: false,
        feedback: null,
        budget: null,
        pending: false,
        choosing: new WeakSet(),
        drag: null,
        over: null,
        tagDraft: "",
        announce: "",
    };
    let picker = null;
    let checkTimer = null;
    let leaving = false;

    const enumIcon = (label) => {
        const src = index.enumImage(label);
        return src === null
            ? ""
            : '<img class="enum-icon" src="' + esc(src) + '" alt="">';
    };

    const textInput = (id, label, value, placeholder, kind) =>
        '<div class="setting-field"><label for="' +
        esc(id) +
        '">' +
        esc(label) +
        "</label>" +
        '<input id="' +
        esc(id) +
        '" class="input" type="' +
        kind +
        '" placeholder="' +
        esc(placeholder) +
        '" value="' +
        esc(value) +
        '"></div>';

    const iconChoice = (label, name, value, options, choose) =>
        '<fieldset class="icon-choice-field"><legend class="label">' +
        esc(label) +
        '</legend><div class="icon-choice">' +
        options
            .map((option, position) => {
                const checked = option === value;
                const id = name + "-" + position;
                return (
                    '<label class="' +
                    (checked
                        ? "icon-choice-option active"
                        : "icon-choice-option") +
                    '" for="' +
                    esc(id) +
                    '"><input type="radio" id="' +
                    esc(id) +
                    '" name="' +
                    esc(name) +
                    '" value="' +
                    esc(option) +
                    '"' +
                    (checked ? ' checked=""' : "") +
                    on({ change: () => choose(option) }) +
                    ">" +
                    enumIcon(option) +
                    "<span>" +
                    esc(option) +
                    "</span></label>"
                );
            })
            .join("") +
        "</div></fieldset>";

    const slotFace = (key, label, diamond, id, open, clear) => {
        const filled = key !== "";
        const cls =
            "slot" +
            (diamond ? " slot-diamond" : "") +
            (filled ? " slot-filled" : "");
        const aria = filled
            ? label + ": " + displayName(key) + ", change"
            : label + ": empty, choose one";
        let face;
        if (!filled)
            face =
                '<span class="slot-plus" aria-hidden="true">' +
                ICON.plus +
                "</span>";
        else if (index.image(key) !== null)
            face =
                '<img src="' +
                esc(index.image(key)) +
                '" alt="" loading="lazy">';
        else
            face =
                '<span class="slot-missing" title="' +
                esc("Zayden has no emoji named " + key) +
                '">?</span>';
        return (
            '<div class="' +
            cls +
            '"><button type="button" class="slot-button"' +
            (id === null ? "" : ' id="' + esc(id) + '"') +
            ' aria-label="' +
            esc(aria) +
            '"' +
            (filled
                ? ' title="' + esc(displayName(key) + " (" + key + ")") + '"'
                : "") +
            on({ click: open }) +
            ">" +
            face +
            "</button>" +
            (filled
                ? '<button type="button" class="slot-clear" aria-label="' +
                  esc("Remove " + label) +
                  '"' +
                  on({ click: clear }) +
                  ">" +
                  ICON.x +
                  "</button>"
                : "") +
            "</div>"
        );
    };

    const keySlot = (label, field, get, set, diamond, picked) =>
        '<div class="slot-cell">' +
        slotFace(
            get(),
            label,
            diamond,
            null,
            () =>
                openPicker({
                    field,
                    title: label,
                    weapon: null,
                    armourSlot: null,
                    taken: [],
                    onPick: (p) => {
                        if (p.kind !== "key") return;
                        set(p.key);
                        if (picked) picked(p.key);
                    },
                }),
            () => {
                set("");
                changed();
            },
        ) +
        '<span class="slot-caption">' +
        esc(label) +
        "</span></div>";

    const keySlotList = (label, field, values, max, list, opts) => {
        const noun = FIELDS[field].noun;
        const len = values.length;
        const slots = values.map((k, i) => [i, k]);
        if (len < max) slots.push([len, ""]);
        let html =
            '<div class="' +
            (opts.round ? "slot-list slot-list-round" : "slot-list") +
            '" role="group" aria-label="' +
            esc(label) +
            '">';
        for (const [i, key] of slots) {
            const faceLabel = label + " " + noun + " " + (i + 1);
            const open = () =>
                openPicker({
                    field,
                    title: label + ": " + noun + " " + (i + 1),
                    weapon: opts.weapon ? opts.weapon() || null : null,
                    armourSlot: null,
                    taken: FIELDS[field].repeats ? [] : [...values],
                    onPick: (p) => {
                        if (p.kind !== "key") return;
                        if (i < values.length) values[i] = p.key;
                        else values.push(p.key);
                    },
                });
            const clear = () => {
                if (i < values.length) values.splice(i, 1);
                changed();
            };
            const face = slotFace(
                key,
                faceLabel,
                false,
                "reorder-" + list + "-" + i,
                open,
                clear,
            );
            if (opts.reorderable && key !== "") {
                const over =
                    ui.over !== null &&
                    ui.over.list === list &&
                    ui.over.index === i;
                const reorder = {
                    list,
                    index: i,
                    len,
                    axis: "row",
                    alt: true,
                    name: displayName(key),
                    move: (from, to) => moveItem(values, from, to),
                };
                html +=
                    '<div class="' +
                    (over ? "slot-drag drop-target" : "slot-drag") +
                    '" draggable="true" title="' +
                    REORDER_HINT +
                    '"' +
                    on({ drag: reorder, drop: reorder, keys: reorder }) +
                    ">" +
                    face +
                    "</div>";
            } else {
                html += face;
            }
        }
        return (
            html +
            '<span class="slot-count" aria-hidden="true">' +
            len +
            "/" +
            max +
            "</span></div>"
        );
    };

    const itemSlot = (label, name, icon, open, clear) => {
        const filled = name !== "";
        const aria = filled
            ? label + ": " + name + ", change"
            : label + ": empty, choose one";
        let face;
        if (!filled)
            face =
                '<span class="slot-plus" aria-hidden="true">' +
                ICON.plus +
                "</span>";
        else if (icon === "")
            face = '<span class="slot-missing" aria-hidden="true">?</span>';
        else face = '<img src="' + esc(icon) + '" alt="" loading="lazy">';
        return (
            '<div class="' +
            (filled ? "slot slot-item slot-filled" : "slot slot-item") +
            '">' +
            '<button type="button" class="slot-button" aria-label="' +
            esc(aria) +
            '" title="' +
            esc(name) +
            '"' +
            on({ click: open }) +
            ">" +
            face +
            "</button>" +
            (clear && filled
                ? '<button type="button" class="slot-clear" aria-label="' +
                  esc("Remove " + label) +
                  '"' +
                  on({ click: clear }) +
                  ">" +
                  ICON.x +
                  "</button>"
                : "") +
            "</div>"
        );
    };

    const alert = (fb) =>
        '<div class="' +
        (fb.ok ? "alert success" : "alert error") +
        '" role="' +
        (fb.ok ? "status" : "alert") +
        '"' +
        (fb.dismissed ? ' hidden=""' : "") +
        "><span>" +
        esc(fb.text) +
        '</span><button type="button" class="alert-dismiss" aria-label="Dismiss"' +
        on({
            click: () => {
                fb.dismissed = true;
                renderForm();
            },
        }) +
        ">" +
        ICON.x +
        "</button></div>";

    function budgetHtml() {
        const b = ui.budget;
        if (b === null) return "";
        if (!b.ok)
            return "<p class=\"loadout-hint\">Couldn't check Discord's limits right now.</p>";
        const c = b.check;
        let html =
            '<div class="budget-meters"><label class="budget-meter"><span>' +
            c.components +
            " / " +
            c.max_components +
            ' Discord components</span><meter min="0" max="' +
            c.max_components +
            '" value="' +
            c.components +
            '" high="' +
            Math.max(c.max_components - 4, 0) +
            '"></meter></label>';
        if (c.text !== null) {
            html +=
                '<label class="budget-meter"><span>~' +
                c.text +
                " / " +
                c.max_text +
                ' characters</span><meter min="0" max="' +
                c.max_text +
                '" value="' +
                c.text +
                '" high="' +
                Math.max(c.max_text - 400, 0) +
                '"></meter></label>';
        }
        html += "</div>";
        if (c.error !== null)
            html += '<p class="budget-error">' + esc(c.error) + "</p>";
        return html;
    }

    function lists() {
        const fragments = 0;
        const perks = fragments + s.aspects.length;
        const mods = perks + s.weapons.length;
        const artifact = mods + s.armour.length;
        return { fragments, perks, mods, artifact, stats: artifact + 1 };
    }

    function subclassCard() {
        const l = lists();
        const scalar = (prop) => [
            () => s[prop],
            (v) => {
                s[prop] = v;
            },
        ];
        const nameSuper = (key) => {
            const known = catalog.super_names.find(([emoji]) => emoji === key);
            s.super_name = known ? known[1] : displayName(key);
        };
        let html =
            '<section class="settings-section loadout-card" aria-labelledby="subclass-heading">' +
            '<h2 id="subclass-heading" class="loadout-card-title">Subclass</h2><div class="subclass-top"><div class="super-field">' +
            keySlot(
                "Super",
                "super",
                ...scalar("super_emoji"),
                true,
                nameSuper,
            ) +
            '</div><div class="ability-row" role="group" aria-label="Abilities">' +
            keySlot(
                "Class ability",
                "classAbility",
                ...scalar("class_ability"),
                false,
            ) +
            keySlot("Jump", "jump", ...scalar("jump"), false) +
            keySlot("Melee", "melee", ...scalar("melee"), false) +
            keySlot("Grenade", "grenade", ...scalar("grenade"), false) +
            '</div></div><div class="aspect-list">';
        s.aspects.forEach((row, i) => {
            html +=
                '<div class="aspect-block">' +
                keySlot(
                    "Aspect",
                    "aspect",
                    () => row.aspect,
                    (v) => {
                        row.aspect = v;
                    },
                    false,
                ) +
                '<div class="aspect-fragments"><span class="label">Fragments</span>' +
                keySlotList(
                    "Fragments",
                    "fragment",
                    row.fragments,
                    MAX.fragments,
                    l.fragments + i,
                    {},
                ) +
                '</div><button type="button" class="btn btn-ghost aspect-remove"' +
                on({
                    click: () => {
                        s.aspects = s.aspects.filter((r) => r !== row);
                        changed();
                    },
                }) +
                ">Remove aspect</button></div>";
        });
        if (s.aspects.length < MAX.aspects) {
            html +=
                '<button type="button" class="btn btn-secondary"' +
                on({
                    click: () => {
                        s.aspects.push({ aspect: "", fragments: [] });
                        changed();
                    },
                }) +
                ">" +
                ICON.plus +
                "Add aspect</button>";
        }
        return html + "</div></section>";
    }

    function weaponPicked(onWeapon) {
        return (p) => {
            if (p.kind !== "weapon") return;
            onWeapon({
                name: p.weapon.name,
                affinity: p.weapon.affinity,
                archetype: p.weapon.archetype,
                icon_url: p.weapon.icon_url,
                perks: [],
            });
        };
    }

    function weaponLine(row, list) {
        const replace = () =>
            openPicker({
                field: "weapon",
                title: "Weapon",
                weapon: null,
                armourSlot: null,
                taken: [],
                onPick: weaponPicked((w) => {
                    if (row.name !== w.name) row.perks = [];
                    row.name = w.name;
                    row.affinity = w.affinity;
                    row.archetype = w.archetype;
                    row.icon_url = w.icon_url;
                    ui.choosing.delete(row);
                }),
            });
        const usual = catalog.weapons.find(
            (w) => w.name === row.name,
        )?.affinity;
        const choosing = ui.choosing.has(row);
        return (
            '<div class="gear-row">' +
            itemSlot("Weapon", row.name, row.icon_url, replace, null) +
            '<div class="gear-info"><span class="gear-name">' +
            esc(row.name) +
            '</span><span class="gear-meta">' +
            '<button type="button" class="gear-affinity" aria-expanded="' +
            choosing +
            '" aria-label="' +
            esc("Damage type: " + row.affinity + ", change") +
            '"' +
            on({
                click: () => {
                    if (choosing) ui.choosing.delete(row);
                    else ui.choosing.add(row);
                    renderForm();
                },
            }) +
            ">" +
            enumIcon(row.affinity) +
            esc(row.affinity) +
            "</button>" +
            esc(row.archetype) +
            (usual !== undefined && usual !== row.affinity
                ? '<span class="gear-hint">' +
                  esc("Normally " + usual) +
                  "</span>"
                : "") +
            "</span>" +
            (choosing
                ? iconChoice(
                      "Damage type",
                      row.affinity,
                      catalog.options.affinities,
                      (v) => {
                          row.affinity = v;
                          ui.choosing.delete(row);
                          changed();
                      },
                  )
                : "") +
            "</div>" +
            keySlotList("Perks", "weaponPerk", row.perks, MAX.perks, list, {
                round: true,
                weapon: () => row.name,
            }) +
            '<button type="button" class="btn btn-ghost gear-remove" aria-label="' +
            esc("Remove " + row.name) +
            '"' +
            on({
                click: () => {
                    s.weapons = s.weapons.filter((r) => r !== row);
                    changed();
                },
            }) +
            ">" +
            ICON.x +
            "</button></div>"
        );
    }

    function armourLine(row, list) {
        const open = () =>
            openPicker({
                field: "armour",
                title: row.slot,
                weapon: null,
                armourSlot: row.slot,
                taken: [],
                onPick: (p) => {
                    if (p.kind !== "armour") return;
                    row.name = p.name;
                    row.icon_url = p.icon_url;
                },
            });
        const clear = () => {
            row.name = "";
            row.icon_url = "";
            changed();
        };
        const empty = row.name === "";
        return (
            '<div class="gear-row">' +
            itemSlot(row.slot, row.name, row.icon_url, open, clear) +
            '<div class="gear-info"><span class="' +
            (empty ? "gear-name gear-empty" : "gear-name") +
            '">' +
            esc(empty ? "Empty" : row.name) +
            '</span><span class="gear-meta">' +
            enumIcon(row.slot) +
            esc(row.slot) +
            "</span></div>" +
            keySlotList("Mods", "armourMod", row.mods, MAX.mods, list, {
                reorderable: true,
            }) +
            "</div>"
        );
    }

    function gearCard() {
        const l = lists();
        const add = () =>
            openPicker({
                field: "weapon",
                title: "Add weapon",
                weapon: null,
                armourSlot: null,
                taken: [],
                onPick: weaponPicked((w) => s.weapons.push(w)),
            });
        let html =
            '<section class="settings-section loadout-card" aria-labelledby="gear-heading">' +
            '<h2 id="gear-heading" class="loadout-card-title">Gear and mods</h2><h3 class="loadout-subtitle">Weapons</h3><div class="gear-list">';
        s.weapons.forEach((row, i) => {
            html += weaponLine(row, l.perks + i);
        });
        if (s.weapons.length < MAX.weapons) {
            html +=
                '<div class="gear-row">' +
                itemSlot("New weapon", "", "", add, null) +
                '<div class="gear-info"><span class="gear-name gear-empty">Add a weapon</span><span class="gear-meta">' +
                s.weapons.length +
                " of " +
                MAX.weapons +
                "</span></div></div>";
        }
        html +=
            '</div><h3 class="loadout-subtitle">Armour</h3><p class="loadout-hint">Leave a slot empty to leave it out of the build.</p><div class="gear-list">';
        s.armour.forEach((row, i) => {
            html += armourLine(row, l.mods + i);
        });
        return html + "</div></section>";
    }

    function artifactCard() {
        const l = lists();
        const name = s.artifact_name;
        const named = name !== "";
        const open = () =>
            openPicker({
                field: "artifact",
                title: "Artifact",
                weapon: null,
                armourSlot: null,
                taken: [],
                onPick: (p) => {
                    if (p.kind === "key") s.artifact_name = p.key;
                },
            });
        return (
            '<section class="settings-section loadout-card" aria-labelledby="artifact-heading">' +
            '<h2 id="artifact-heading" class="loadout-card-title">Artifact</h2><div class="artifact-row">' +
            '<button type="button" class="' +
            (named ? "artifact-choice" : "artifact-choice artifact-empty") +
            '" aria-label="' +
            esc(
                named
                    ? "Artifact: " + name + ", change"
                    : "Artifact: none, choose one",
            ) +
            '"' +
            on({ click: open }) +
            ">" +
            esc(named ? name : "Choose artifact") +
            "</button>" +
            (named
                ? '<button type="button" class="btn btn-ghost"' +
                  on({
                      click: () => {
                          s.artifact_name = "";
                          changed();
                      },
                  }) +
                  ">Clear</button>"
                : "") +
            '</div><span class="label">Artifact perks</span>' +
            keySlotList(
                "Artifact perks",
                "artifactPerk",
                s.artifact_perks,
                MAX.artifactPerks,
                l.artifact,
                { reorderable: true },
            ) +
            "</section>"
        );
    }

    function statsCard() {
        const list = lists().stats;
        let html =
            '<section class="settings-section loadout-card" aria-labelledby="stats-heading">' +
            '<h2 id="stats-heading" class="loadout-card-title">Stat priority</h2><p class="loadout-hint">Highest priority first. ' +
            "Drag a row, or focus its handle and use the arrow keys, to reorder. Leave a value blank to leave the stat out.</p>" +
            '<div class="stat-list">';
        s.stats.forEach((row, i) => {
            const over =
                ui.over !== null &&
                ui.over.list === list &&
                ui.over.index === i;
            const reorder = {
                list,
                index: i,
                len: s.stats.length,
                axis: "column",
                alt: false,
                name: row.stat,
                move: (from, to) => moveItem(s.stats, from, to),
            };
            html +=
                '<div class="' +
                (over ? "stat-row drop-target" : "stat-row") +
                '"' +
                on({ drop: reorder }) +
                ">" +
                '<button type="button" class="stat-handle" id="reorder-' +
                list +
                "-" +
                i +
                '" draggable="true" aria-label="' +
                esc("Move " + row.stat + ", priority " + (i + 1)) +
                '"' +
                on({ drag: reorder, keys: reorder }) +
                ">" +
                ICON.grip +
                '</button><span class="stat-rank" aria-hidden="true">' +
                (i + 1) +
                "</span>" +
                enumIcon(row.stat) +
                '<span class="stat-name">' +
                esc(row.stat) +
                "</span>" +
                textInput(
                    "stat-value-" + i,
                    "Value",
                    row.value,
                    "0–200",
                    "number",
                ) +
                "</div>";
        });
        return html + "</div></section>";
    }

    function addTag() {
        const key = trim(ui.tagDraft);
        if (key === "") return;
        if (s.tags.length < MAX.tags) s.tags.push(key);
        ui.tagDraft = "";
        changed();
    }

    function detailsCard() {
        let chips = "";
        s.tags.forEach((tag, i) => {
            chips +=
                '<span class="chip"><span class="chip-label">' +
                esc(tag) +
                '</span><button type="button" class="chip-remove" aria-label="' +
                esc("Remove tag " + tag) +
                '"' +
                on({
                    click: () => {
                        if (i < s.tags.length) s.tags.splice(i, 1);
                        changed();
                    },
                }) +
                ">" +
                ICON.x +
                "</button></span>";
        });
        return (
            '<section class="settings-section loadout-card" aria-labelledby="details-heading">' +
            '<h2 id="details-heading" class="loadout-card-title">Details</h2><div class="loadout-grid">' +
            textInput("loadout-author", "Author", s.author, "", "text") +
            textInput(
                "loadout-dim",
                "DIM link",
                s.dim_link,
                "https://dim.gg/…",
                "url",
            ) +
            textInput(
                "loadout-video",
                "Video link",
                s.video_url,
                "https://youtu.be/…",
                "url",
            ) +
            '</div><div class="setting-field"><label for="loadout-tag">Tags (up to 3)</label><div class="chip-list">' +
            chips +
            '</div><div class="chip-add"><input class="input" id="loadout-tag"' +
            (ui.tagDraft === "" ? "" : ' value="' + esc(ui.tagDraft) + '"') +
            ">" +
            '<button type="button" class="btn btn-secondary"' +
            (s.tags.length >= MAX.tags ? ' disabled=""' : "") +
            on({ click: addTag }) +
            ">Add</button></div></div>" +
            '<div class="setting-field"><label for="loadout-how">How it works</label><textarea id="loadout-how" class="input" rows="8">\n' +
            esc(s.how_it_works) +
            "</textarea></div></section>"
        );
    }

    function missing() {
        if (!index.hasAny()) return [];
        const keys = emojiKeys(s).filter((k) => !index.hasEmoji(k));
        return [...new Set(keys)].sort(byKey);
    }

    function formHtml() {
        const o = catalog.options;
        const set = (prop) => (v) => {
            s[prop] = v;
            changed();
        };
        let html =
            '<header class="loadout-header"><h1>' +
            (s.id !== null ? "Edit loadout" : "New loadout") +
            "</h1>" +
            textInput("loadout-name", "Build name", s.name, "", "text") +
            '<div class="loadout-choices">' +
            iconChoice("Class", "loadout-class", s.class, o.classes, set("class")) +
            iconChoice(
                "Subclass",
                "loadout-element",
                s.element,
                o.elements,
                set("element"),
            ) +
            iconChoice("Mode", "loadout-mode", s.mode, o.modes, set("mode")) +
            '</div><div class="budget" role="status" aria-live="polite">' +
            budgetHtml() +
            "</div></header>";
        if (ui.feedback !== null) html += alert(ui.feedback);
        if (ui.restored || !same(s, saved)) {
            html +=
                '<div class="draft-banner" role="status"><span>' +
                (ui.restored
                    ? "Restored your unsaved changes from this tab."
                    : "Unsaved changes") +
                "</span>" +
                '<button type="button" class="btn btn-ghost"' +
                on({ click: discard }) +
                ">Discard changes</button></div>";
        }
        if (!index.hasAny()) {
            html +=
                '<p class="loadout-warning" role="status">Zayden\'s emoji list couldn\'t be loaded, so icons are hidden. You can still edit and save.</p>';
        }
        const gone = missing();
        if (gone.length > 0) {
            html +=
                '<p class="loadout-warning" role="status">' +
                esc(
                    "Zayden has no emoji for these, so /destiny2 builds can't show this build until they're added: " +
                        gone.join(", "),
                ) +
                "</p>";
        }
        html +=
            '<div class="loadout-board">' +
            subclassCard() +
            gearCard() +
            artifactCard() +
            statsCard() +
            detailsCard() +
            "</div>";
        html +=
            '<p class="visually-hidden" role="status">' +
            esc(ui.announce) +
            "</p>";
        html +=
            '<div class="form-actions"><button type="submit" class="btn btn-primary"' +
            (ui.pending ? ' disabled=""' : "") +
            on({}) +
            ">" +
            (ui.pending ? "Saving…" : "Save") +
            "</button></div>";
        return html;
    }

    function pickerResults() {
        const r = picker.request;
        const scope = {
            field: r.field,
            class: s.class,
            element: s.element,
            weapon: r.weapon,
            armourSlot: r.armourSlot,
            taken: r.taken,
        };
        return sections(catalog, index, scope, picker.query);
    }

    function pickerHtml() {
        if (picker === null) return "";
        const r = picker.request;
        const noun = FIELDS[r.field].noun;
        let html =
            '<div class="picker-panel"><header class="picker-header"><h2 id="picker-title" class="picker-title">' +
            esc(r.title) +
            '</h2><button type="button" class="icon-button" aria-label="Close"' +
            on({ click: closePicker }) +
            ">" +
            ICON.x +
            "</button></header>";
        if (picker.create !== null) return html + createHtml() + "</div>";

        const groups = pickerResults();
        picker.flat = groups.flatMap((g) => g.items);
        const total = picker.flat.length;
        html +=
            '<div class="picker-search"><input class="input" type="search" role="combobox" aria-expanded="true" aria-controls="picker-results"' +
            ' aria-activedescendant="picker-option-' +
            picker.active +
            '" aria-label="' +
            esc("Search " + noun + "s") +
            '" placeholder="' +
            esc("Search " + noun + "s…") +
            '" value="' +
            esc(picker.query) +
            '"></div><ul id="picker-results" class="picker-results" role="listbox">';
        let n = 0;
        for (const group of groups) {
            html +=
                '<li role="presentation" class="picker-section"><span class="picker-section-title">' +
                esc(group.title) +
                '</span><ul role="group" class="picker-group">';
            for (const c of group.items) {
                const i = n;
                const active = picker.active === i;
                html +=
                    '<li id="picker-option-' +
                    i +
                    '" role="option" aria-selected="' +
                    active +
                    '" aria-disabled="' +
                    c.selected +
                    '" class="picker-option' +
                    (active ? " active" : "") +
                    (c.selected ? " taken" : "") +
                    '"' +
                    on({ click: () => choose(i), hover: () => setActive(i) }) +
                    '><span class="picker-thumb">' +
                    (c.image === null
                        ? '<span class="slot-missing" aria-hidden="true">?</span>'
                        : '<img src="' +
                          esc(c.image) +
                          '" alt="" loading="lazy">') +
                    '</span><span class="picker-label">' +
                    esc(c.label) +
                    '</span><span class="picker-detail">' +
                    esc(c.detail) +
                    "</span>" +
                    (c.selected
                        ? '<span class="picker-tag">Selected</span>'
                        : "") +
                    "</li>";
                n += 1;
            }
            html += "</ul></li>";
        }
        const q = trim(picker.query);
        const createActive = picker.active === total;
        html +=
            '<li id="picker-option-' +
            total +
            '" role="option" aria-selected="' +
            createActive +
            '" class="picker-option picker-create' +
            (createActive ? " active" : "") +
            '"' +
            on({ click: () => startCreate() }) +
            '><span class="picker-thumb">' +
            ICON.plus +
            '</span><span class="picker-label">' +
            esc(
                q === ""
                    ? "Create new " + noun + "…"
                    : "Create new “" + q + "”…",
            ) +
            "</span></li></ul>";
        return html + "</div>";
    }

    const preview = (src) =>
        src === null
            ? ""
            : '<figure class="picker-preview"><img src="' +
              esc(src) +
              '" alt=""><figcaption>Preview</figcaption></figure>';

    const nameField = (id, label, value) =>
        '<div class="setting-field"><label for="' +
        id +
        '">' +
        label +
        '</label><input id="' +
        id +
        '" class="input" value="' +
        esc(value) +
        '"></div>';

    const iconUrlField = (id, value) =>
        '<div class="setting-field"><label for="' +
        id +
        '">Icon link</label><input id="' +
        id +
        '" class="input" type="url" inputmode="url" placeholder="https://www.bungie.net/common/destiny2_content/icons/…" value="' +
        esc(value) +
        '"><p class="field-hint">An https link to the item\'s icon, e.g. from light.gg or DIM.</p></div>';

    const optionSelect = (id, label, value, options) =>
        '<div class="setting-field"><label for="' +
        id +
        '">' +
        label +
        '</label><select id="' +
        id +
        '" class="input">' +
        options
            .map(
                (o) =>
                    '<option value="' +
                    esc(o) +
                    '"' +
                    (o === value ? ' selected=""' : "") +
                    ">" +
                    esc(o) +
                    "</option>",
            )
            .join("") +
        "</select></div>";

    const action = (label, blocked, done) =>
        '<div class="picker-actions"><button type="button" class="btn btn-primary"' +
        (blocked ? ' disabled=""' : "") +
        on({ click: done }) +
        ">" +
        label +
        "</button></div>";

    function keyProblem(key) {
        if (!isValidKey(key))
            return "Use 2–32 lowercase letters, digits or underscores.";
        if (index.reserved.has(key))
            return "That name belongs to a built-in class, element, weapon or stat icon.";
        if (index.hasEmoji(key))
            return "Zayden already has this emoji. Go back and pick it from the list.";
        return null;
    }

    function emojiSource(c) {
        if (c.fromFile) return c.file === null ? null : { DataUri: c.file };
        const u = trim(c.url);
        return u === "" ? null : { Url: u };
    }

    function createHtml() {
        const c = picker.create;
        let html =
            '<div class="picker-create-form"><button type="button" class="btn btn-ghost picker-back"' +
            on({
                click: () => {
                    picker.create = null;
                    renderPicker();
                },
            }) +
            ">Back to results</button>";
        if (c.kind === "emoji") {
            const problem = keyProblem(c.key);
            const blocked =
                c.pending || problem !== null || emojiSource(c) === null;
            html +=
                '<div class="setting-field"><label for="new-emoji-name">Emoji name</label><input id="new-emoji-name" class="input" autocomplete="off"' +
                ' spellcheck="false" aria-describedby="new-emoji-name-hint" aria-invalid="' +
                (problem !== null) +
                '" value="' +
                esc(c.key) +
                '">' +
                '<p id="new-emoji-name-hint" class="' +
                (problem !== null ? "field-hint field-error" : "field-hint") +
                '">' +
                esc(problem ?? "Shows as “" + displayName(c.key) + "”.") +
                "</p></div>" +
                '<div class="segmented" role="tablist" aria-label="Image source">' +
                '<button type="button" role="tab" aria-selected="' +
                !c.fromFile +
                '" class="' +
                (c.fromFile ? "seg" : "seg active") +
                '"' +
                on({
                    click: () => {
                        c.fromFile = false;
                        renderPicker();
                    },
                }) +
                ">Image link</button>" +
                '<button type="button" role="tab" aria-selected="' +
                c.fromFile +
                '" class="' +
                (c.fromFile ? "seg active" : "seg") +
                '"' +
                on({
                    click: () => {
                        c.fromFile = true;
                        renderPicker();
                    },
                }) +
                '>Upload file</button></div><div class="setting-field">' +
                (c.fromFile
                    ? '<label for="new-emoji-file">Image file</label><input id="new-emoji-file" class="input" type="file" accept="image/png,image/jpeg,image/gif,image/webp" aria-describedby="new-emoji-image-hint">'
                    : '<label for="new-emoji-url">Image link</label><input id="new-emoji-url" class="input" type="url" inputmode="url" placeholder="https://…" aria-describedby="new-emoji-image-hint" value="' +
                      esc(c.url) +
                      '">') +
                '<p id="new-emoji-image-hint" class="field-hint">PNG, JPEG, GIF or WebP, up to 256 KiB. Square images look best.</p></div>' +
                preview(c.fromFile ? c.file : httpsPreview(c.url)) +
                (c.fileError === null
                    ? ""
                    : '<p class="alert error" role="alert">' +
                      esc(c.fileError) +
                      "</p>") +
                (c.error === null
                    ? ""
                    : '<p class="alert error" role="alert">' +
                      esc(c.error) +
                      "</p>") +
                action(c.pending ? "Creating…" : "Create emoji", blocked, () =>
                    createEmoji(c),
                );
        } else if (c.kind === "weapon") {
            const blocked =
                trim(c.name) === "" || httpsPreview(c.icon) === null;
            html +=
                '<p class="field-hint">Saved to the weapon catalog when you save this loadout.</p>' +
                nameField("new-weapon-name", "Weapon name", c.name) +
                optionSelect(
                    "new-weapon-affinity",
                    "Damage type",
                    c.affinity,
                    catalog.options.affinities,
                ) +
                optionSelect(
                    "new-weapon-archetype",
                    "Weapon type",
                    c.archetype,
                    catalog.options.archetypes,
                ) +
                iconUrlField("new-weapon-icon", c.icon) +
                preview(httpsPreview(c.icon)) +
                action("Use this weapon", blocked, () =>
                    finish({
                        kind: "weapon",
                        weapon: {
                            name: trim(c.name),
                            affinity: c.affinity,
                            archetype: c.archetype,
                            icon_url: trim(c.icon),
                            known_perks: [],
                        },
                    }),
                );
        } else if (c.kind === "armour") {
            const blocked =
                trim(c.name) === "" || httpsPreview(c.icon) === null;
            html +=
                nameField("new-armour-name", "Armour name", c.name) +
                iconUrlField("new-armour-icon", c.icon) +
                preview(httpsPreview(c.icon)) +
                action("Use this armour", blocked, () =>
                    finish({
                        kind: "armour",
                        name: trim(c.name),
                        icon_url: trim(c.icon),
                    }),
                );
        } else {
            html +=
                nameField("new-artifact-name", "Artifact name", c.name) +
                action("Use this artifact", trim(c.name) === "", () =>
                    finish({ kind: "key", key: trim(c.name) }),
                );
        }
        return html + "</div>";
    }

    function renderForm() {
        handlerList = [];
        morphChildren(form, parse(formHtml()));
    }

    function renderPicker() {
        handlerList = [];
        morphChildren(dialog, parse(pickerHtml()));
    }

    function render() {
        renderForm();
        renderPicker();
    }

    function persist() {
        if (same(s, saved)) clearDraft(draftKey);
        else storeDraft(draftKey, s);
    }

    async function postJson(url, body) {
        const response = await fetch(url, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
                Accept: "application/json",
            },
            body: JSON.stringify(body),
            credentials: "same-origin",
        });
        let reply = null;
        try {
            reply = await response.json();
        } catch {
            reply = null;
        }
        if (
            reply !== null &&
            typeof reply === "object" &&
            ("ok" in reply || "error" in reply)
        )
            return reply;
        throw new Error(response.status + " " + response.statusText);
    }

    function scheduleCheck() {
        if (checkTimer !== null) clearTimeout(checkTimer);
        const snapshot = canon(s);
        checkTimer = setTimeout(async () => {
            checkTimer = null;
            try {
                const reply = await postJson(CHECK_URL, snapshot);
                ui.budget =
                    "ok" in reply
                        ? { ok: true, check: reply.ok }
                        : { ok: false };
            } catch {
                ui.budget = { ok: false };
            }
            renderForm();
        }, CHECK_DELAY);
    }

    function changed() {
        persist();
        scheduleCheck();
        render();
    }

    function discard() {
        clearDraft(draftKey);
        s = canon(saved);
        ui.restored = false;
        ui.choosing = new WeakSet();
        changed();
    }

    async function save() {
        if (ui.pending) return;
        ui.pending = true;
        renderForm();
        let feedback;
        try {
            const reply = await postJson(SAVE_URL, canon(s));
            if ("ok" in reply) {
                clearDraft(draftKey);
                saved = canon(s);
                ui.restored = false;
                feedback = { ok: true, text: "Saved.", dismissed: false };
                if (s.id === null) {
                    leaving = true;
                    window.location.assign(
                        "/admin/destiny2/loadouts/" + reply.ok,
                    );
                    return;
                }
            } else {
                feedback = {
                    ok: false,
                    text: "Failed to save: " + reply.error,
                    dismissed: false,
                };
            }
        } catch (e) {
            feedback = {
                ok: false,
                text:
                    "Failed to save: error reaching server to call server function: " +
                    e.message,
                dismissed: false,
            };
        }
        ui.feedback = feedback;
        ui.pending = false;
        renderForm();
    }

    function openPicker(request) {
        picker = {
            request,
            query: "",
            active: 0,
            create: null,
            flat: [],
            opener: document.activeElement,
        };
        renderPicker();
        if (!dialog.open) dialog.showModal();
        requestAnimationFrame(() =>
            dialog.querySelector(".picker-search input")?.focus(),
        );
    }

    function closePicker() {
        if (dialog.open) dialog.close();
        else onClosed();
    }

    function onClosed() {
        if (picker === null || dialog.open) return;
        const opener = picker.opener;
        picker = null;
        renderPicker();
        if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    }

    function finish(picked) {
        if (picker === null) return;
        picker.request.onPick(picked);
        closePicker();
        changed();
    }

    function resolve(field, key) {
        if (field === "weapon") {
            const w = catalog.weapons.find((x) => x.name === key);
            return w ? { kind: "weapon", weapon: { ...w } } : null;
        }
        if (field === "armour") {
            const a = catalog.armour.find((x) => x.name === key);
            return a
                ? { kind: "armour", name: a.name, icon_url: a.icon_url }
                : null;
        }
        return { kind: "key", key };
    }

    function choose(i) {
        const item = picker.flat[i];
        if (item === undefined) {
            startCreate();
            return;
        }
        if (item.selected) return;
        const picked = resolve(picker.request.field, item.key);
        if (picked !== null) picker.request.onPick(picked);
        closePicker();
        changed();
    }

    function scrollActive() {
        if (picker !== null)
            document
                .getElementById("picker-option-" + picker.active)
                ?.scrollIntoView(false);
    }

    function setActive(i) {
        if (picker === null || picker.active === i) return;
        picker.active = i;
        renderPicker();
        scrollActive();
    }

    function startCreate() {
        const field = picker.request.field;
        const query = picker.query;
        if (FIELDS[field].emoji) {
            picker.create = {
                kind: "emoji",
                key: keyFromQuery(query),
                fromFile: false,
                url: "",
                file: null,
                fileError: null,
                pending: false,
                error: null,
            };
        } else if (field === "weapon") {
            picker.create = {
                kind: "weapon",
                name: trim(query),
                affinity: catalog.options.affinities[0] ?? "",
                archetype: catalog.options.archetypes[0] ?? "",
                icon: "",
            };
        } else if (field === "armour") {
            picker.create = { kind: "armour", name: trim(query), icon: "" };
        } else {
            picker.create = { kind: "artifact", name: trim(query) };
        }
        renderPicker();
    }

    async function createEmoji(c) {
        const source = emojiSource(c);
        if (source === null) return;
        c.pending = true;
        renderPicker();
        let reply;
        try {
            reply = await postJson(EMOJI_URL, { name: c.key, source });
        } catch (e) {
            reply = {
                error:
                    "error reaching server to call server function: " +
                    e.message,
            };
        }
        c.pending = false;
        if ("ok" in reply) {
            catalog.emojis.push(reply.ok);
            index = buildIndex(catalog);
            if (picker !== null && picker.create === c)
                finish({ kind: "key", key: reply.ok.name });
            else render();
        } else {
            c.error = reply.error;
            renderPicker();
        }
    }

    function readImage(input, c) {
        const chosen = input.files?.[0];
        if (!chosen) {
            c.file = null;
            renderPicker();
            return;
        }
        if (chosen.size > MAX_IMAGE_BYTES) {
            c.file = null;
            c.fileError = "That file is over 256 KiB. Choose a smaller image.";
            renderPicker();
            return;
        }
        const reader = new FileReader();
        reader.onload = () => {
            if (typeof reader.result === "string") {
                c.fileError = null;
                c.file = reader.result;
            } else {
                c.fileError = "That file couldn't be read.";
            }
            renderPicker();
        };
        reader.onerror = () => {
            c.fileError = "That file couldn't be read.";
            renderPicker();
        };
        reader.readAsDataURL(chosen);
    }

    function handlerOf(target, root, prop) {
        for (
            let el = target instanceof Element ? target : target?.parentElement;
            el && el !== root;
            el = el.parentElement
        ) {
            const h = handlers.get(el);
            if (h && h[prop]) return h;
        }
        return null;
    }

    const FORM_FIELDS = {
        "loadout-name": "name",
        "loadout-author": "author",
        "loadout-dim": "dim_link",
        "loadout-video": "video_url",
        "loadout-how": "how_it_works",
    };

    form.addEventListener("click", (e) => {
        const h = handlerOf(e.target, form, "click");
        if (h) h.click();
    });

    form.addEventListener("change", (e) => {
        const h = handlerOf(e.target, form, "change");
        if (h) h.change();
    });

    form.addEventListener("input", (e) => {
        const el = e.target;
        if (el.id in FORM_FIELDS) s[FORM_FIELDS[el.id]] = el.value;
        else if (el.id.startsWith("stat-value-")) {
            const row = s.stats[Number(el.id.slice("stat-value-".length))];
            if (row) row.value = el.value;
        } else if (el.matches(".chip-add input")) {
            ui.tagDraft = el.value;
            return;
        } else return;
        changed();
    });

    form.addEventListener("keydown", (e) => {
        if (e.target.matches?.(".chip-add input") && e.key === "Enter") {
            e.preventDefault();
            addTag();
            return;
        }
        const h = handlerOf(e.target, form, "keys");
        if (!h) return;
        const r = h.keys;
        if (r.alt && !e.altKey) return;
        const [back, forward] =
            r.axis === "row"
                ? ["ArrowLeft", "ArrowRight"]
                : ["ArrowUp", "ArrowDown"];
        let to;
        if (e.key === back) to = r.index > 0 ? r.index - 1 : null;
        else if (e.key === forward)
            to = r.index + 1 < r.len ? r.index + 1 : null;
        else return;
        e.preventDefault();
        if (to === null) return;
        r.move(r.index, to);
        ui.announce = r.name + " moved to position " + (to + 1) + " of " + r.len + ".";
        changed();
        requestAnimationFrame(() =>
            document.getElementById("reorder-" + r.list + "-" + to)?.focus(),
        );
    });

    form.addEventListener("dragstart", (e) => {
        const h = handlerOf(e.target, form, "drag");
        if (!h) return;
        ui.drag = {
            list: h.drag.list,
            from: h.drag.index,
            len: h.drag.len,
            name: h.drag.name,
            move: h.drag.move,
        };
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = "move";
            e.dataTransfer.setData("text/plain", "");
        }
    });

    form.addEventListener("dragover", (e) => {
        const h = handlerOf(e.target, form, "drop");
        if (!h || ui.drag === null || ui.drag.list !== h.drop.list) return;
        e.preventDefault();
        if (
            ui.over === null ||
            ui.over.list !== h.drop.list ||
            ui.over.index !== h.drop.index
        ) {
            ui.over = { list: h.drop.list, index: h.drop.index };
            renderForm();
        }
    });

    form.addEventListener("drop", (e) => {
        const h = handlerOf(e.target, form, "drop");
        if (!h) return;
        e.preventDefault();
        const drag = ui.drag;
        ui.drag = null;
        ui.over = null;
        if (drag !== null && drag.list === h.drop.list) {
            drag.move(drag.from, h.drop.index);
            ui.announce =
                drag.name +
                " moved to position " +
                (h.drop.index + 1) +
                " of " +
                drag.len +
                ".";
            changed();
        } else {
            renderForm();
        }
    });

    form.addEventListener("dragend", () => {
        if (ui.drag === null && ui.over === null) return;
        ui.drag = null;
        ui.over = null;
        renderForm();
    });

    window.addEventListener("beforeunload", (e) => {
        if (leaving || same(s, saved)) return;
        e.preventDefault();
        e.returnValue = "";
    });

    // A page restored from the back/forward cache after a create would still
    // hold the pending save; load it afresh instead.
    window.addEventListener("pageshow", (e) => {
        if (e.persisted && leaving) window.location.reload();
    });

    form.addEventListener("submit", (e) => {
        e.preventDefault();
        save();
    });

    dialog.addEventListener("click", (e) => {
        if (e.target === dialog) {
            closePicker();
            return;
        }
        const h = handlerOf(e.target, dialog, "click");
        if (h) h.click();
    });

    dialog.addEventListener("mousemove", (e) => {
        const h = handlerOf(e.target, dialog, "hover");
        if (h) h.hover();
    });

    dialog.addEventListener("close", onClosed);

    dialog.addEventListener("input", (e) => {
        if (picker === null) return;
        const el = e.target;
        const c = picker.create;
        if (el.matches(".picker-search input")) {
            picker.query = el.value;
            picker.active = 0;
        } else if (c !== null && el.id === "new-emoji-name") c.key = el.value;
        else if (c !== null && el.id === "new-emoji-url") c.url = el.value;
        else if (
            c !== null &&
            (el.id === "new-weapon-name" ||
                el.id === "new-armour-name" ||
                el.id === "new-artifact-name")
        )
            c.name = el.value;
        else if (
            c !== null &&
            (el.id === "new-weapon-icon" || el.id === "new-armour-icon")
        )
            c.icon = el.value;
        else return;
        renderPicker();
        if (el.matches(".picker-search input")) scrollActive();
    });

    dialog.addEventListener("change", (e) => {
        if (picker === null || picker.create === null) return;
        const el = e.target;
        const c = picker.create;
        if (el.id === "new-emoji-file") readImage(el, c);
        else if (el.id === "new-weapon-affinity") c.affinity = el.value;
        else if (el.id === "new-weapon-archetype") c.archetype = el.value;
        else return;
        renderPicker();
    });

    dialog.addEventListener("keydown", (e) => {
        if (picker === null || !e.target.matches?.(".picker-search input"))
            return;
        const last = picker.flat.length;
        if (e.key === "ArrowDown") {
            e.preventDefault();
            setActive(Math.min(picker.active + 1, last));
        } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setActive(Math.max(picker.active - 1, 0));
        } else if (e.key === "Enter") {
            e.preventDefault();
            choose(picker.active);
        }
    });

    changes = 0;
    diffs = [];
    render();
    const initialChanges = changes;
    const initialDiffs = [...diffs];

    const draft = loadDraft(draftKey);
    if (draft !== null && !same(draft, saved)) {
        s = draft;
        ui.restored = true;
    }
    persist();
    scheduleCheck();
    render();

    form.loadoutEditor = {
        initialChanges,
        initialDiffs,
        draftKey,
        state: () => canon(s),
        saved: () => canon(saved),
        picker: () =>
            picker === null
                ? null
                : {
                      title: picker.request.title,
                      field: picker.request.field,
                      query: picker.query,
                      active: picker.active,
                      creating: picker.create !== null,
                  },
        helpers,
    };
}
