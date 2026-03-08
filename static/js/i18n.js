/* ─────────────────────────────────────────────
   Proxy Pulse — Internationalization (i18n)
   ───────────────────────────────────────────── */

const I18N = {
    _locale: 'en',
    _translations: {},
    _listeners: [],

    get locale() { return this._locale; },

    async init() {
        const saved = localStorage.getItem('pp-lang');
        const lang = saved || (navigator.language.startsWith('zh') 
            ? (navigator.language.includes('TW') || navigator.language.includes('HK') ? 'zh-TW' : 'zh-CN')
            : navigator.language.startsWith('ja') ? 'ja' : 'en');
        await this.setLocale(lang);
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
    },

    onChange(fn) {
        this._listeners.push(fn);
    }
};
