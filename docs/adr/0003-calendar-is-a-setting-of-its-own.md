# The calendar is a setting of its own

Until v0.3.0 the language decided the calendar: Persian meant Jalali dates, Persian digits and Saturday weeks. With Arabic, Russian and Chinese added, we split the calendar out into its own setting (Gregorian, Jalali, Hijri Umm al-Qura, Chinese lunar, or Automatic, which follows the language), because people who read one language often date things by another calendar. The calendar, not the language, now owns the digits of every number, the first day of the week (Saturday for Jalali and Hijri, Monday for Chinese lunar; Gregorian alone takes it from the language), and the boundaries of This month.

## Consequences

- This month follows the chosen calendar, so under Jalali, Hijri or lunar it no longer lines up with Anthropic's billing month. The extra-usage spend can't be recomputed for another month, so it stays the billing-month figure and is labelled as such instead of being silently mixed in.
- A Chinese lunar month can be a leap month, so a month's length and name are irregular.
- Notifications written by the core follow the calendar's digits too, so the core needs the same digit rules as the window.
