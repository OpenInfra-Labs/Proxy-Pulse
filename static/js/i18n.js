/* ─────────────────────────────────────────────
   Proxy Pulse — Internationalization (i18n)
   ───────────────────────────────────────────── */

const I18N = {
    _locale: 'en',
    _translations: {},
    _listeners: [],
    ready: Promise.resolve(),

    get locale() { return this._locale; },

    async init() {
        const saved = localStorage.getItem('pp-lang');
        const lang = saved || (navigator.language.startsWith('zh') 
            ? (navigator.language.includes('TW') || navigator.language.includes('HK') ? 'zh-TW' : 'zh-CN')
            : navigator.language.startsWith('ja') ? 'ja' : 'en');
        this.ready = this.setLocale(lang);
        return this.ready;
    },

    async setLocale(locale) {
        if (!['en', 'zh-CN', 'zh-TW', 'ja'].includes(locale)) locale = 'en';
        try {
            const res = await fetch(`/static/i18n/${locale}.json`);
            if (!res.ok) throw new Error(`HTTP ${res.status}`);
            this._translations = await res.json();
        } catch (e) {
            console.warn(`Failed to load locale ${locale}, falling back to en`, e);
            if (locale !== 'en') {
                const res = await fetch('/static/i18n/en.json');
                this._translations = await res.json();
                locale = 'en';
            }
        }
        this._locale = locale;
        localStorage.setItem('pp-lang', locale);
        document.documentElement.lang = locale;
        this.applyAll();
        this._listeners.forEach(fn => fn(locale));
    },

    t(key) {
        const keys = key.split('.');
        let val = this._translations;
        for (const k of keys) {
            if (val && typeof val === 'object' && k in val) {
                val = val[k];
            } else {
                return key; // fallback to key
            }
        }
        return typeof val === 'string' ? val : key;
    },

    applyAll() {
        document.querySelectorAll('[data-i18n]').forEach(el => {
            const key = el.getAttribute('data-i18n');
            el.textContent = this.t(key);
        });
        document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
            el.placeholder = this.t(el.getAttribute('data-i18n-placeholder'));
        });
        document.querySelectorAll('[data-i18n-title]').forEach(el => {
            el.title = this.t(el.getAttribute('data-i18n-title'));
        });
        document.querySelectorAll('[data-i18n-html]').forEach(el => {
            el.innerHTML = this.t(el.getAttribute('data-i18n-html'));
        });
        // Update page title 
        const titleKey = document.querySelector('title')?.getAttribute('data-i18n');
        if (titleKey) document.title = this.t(titleKey);
        document.documentElement.classList.add('i18n-ready');
    },

    onChange(fn) {
        this._listeners.push(fn);
    }
};

/* ─────────────────────────────────────────────
   Timezone-aware date formatting utility
   ───────────────────────────────────────────── */
const TZ = {
    _zone: 'auto',

    get zone() {
        return this._zone === 'auto'
            ? Intl.DateTimeFormat().resolvedOptions().timeZone
            : this._zone;
    },

    set(tz) {
        this._zone = tz || 'auto';
        localStorage.setItem('pp-tz', this._zone);
    },

    init() {
        this._zone = localStorage.getItem('pp-tz') || 'auto';
    },

    /** Format a UTC datetime string (from server) into the user's timezone.
     *  @param {string} utcStr - datetime string from server (assumed UTC if no 'Z')
     *  @param {object} [opts] - Intl.DateTimeFormat options override
     *  @returns {string} formatted date string, or '—' if input is falsy
     */
    fmt(utcStr, opts) {
        if (!utcStr) return '—';
        // Ensure the string is treated as UTC
        let str = utcStr.trim();
        if (!str.endsWith('Z') && !str.includes('+') && !str.includes('T')) {
            str = str.replace(' ', 'T') + 'Z';
        } else if (!str.endsWith('Z') && !str.includes('+')) {
            str += 'Z';
        }
        const d = new Date(str);
        if (isNaN(d.getTime())) return utcStr;
        const defaults = { year: 'numeric', month: '2-digit', day: '2-digit',
                           hour: '2-digit', minute: '2-digit', second: '2-digit',
                           hour12: false, timeZone: this.zone };
        return d.toLocaleString(undefined, Object.assign(defaults, opts || {}));
    },

    /** Format date-only */
    fmtDate(utcStr) {
        return this.fmt(utcStr, { hour: undefined, minute: undefined, second: undefined });
    },

    /** Format time-only (for "last update" style) */
    fmtTime(date) {
        if (!(date instanceof Date)) date = new Date();
        return date.toLocaleTimeString(undefined, { hour12: false, timeZone: this.zone });
    }
};
