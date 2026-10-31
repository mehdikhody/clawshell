# 10: Persian and RTL

**What to build:** A Language picker in Settings offers English and فارسی. Choosing Persian switches every existing screen's strings to `fa` and flips the document to right-to-left, including the top bar, the tab bar and rows. Numbers, percentages, dollars and dates use Persian digits. The choice persists. Later tickets add their own `fa` strings as they add UI. See spec: Platform and stack (RTL).

**Blocked by:** 02

**Status:** done

- [x] The language picker switches between en and fa at runtime and persists
- [x] fa sets the RTL direction and mirrors the layout correctly
- [x] Numbers, percentages, currency and dates are formatted with Persian digits in fa
- [x] All strings existing at the time are translated in both locales
- [x] UI tests cover RTL direction and Persian-digit formatting
