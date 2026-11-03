# clawshell

clawshell runs several Claude Desktop sign-ins side by side on one Windows machine and shows their limits and cost together.

**Disclaimer**:
The statement of what clawshell is and does, which a user accepts before clawshell starts working. Each wording has a version, and a new version must be accepted again.
_Avoid_: Terms, EULA, consent

## Accounts

**Account**:
One Claude Desktop sign-in that clawshell lists, with its own profile, name and badge colour.
_Avoid_: User, login, instance

**Default account**:
The account backed by Claude's own profile, the one Claude's Start-menu entry opens. Listed first as "Claude" until moved.
_Avoid_: Main account, original profile

**Unavailable account**:
An account that is limited, or whose profile is missing. Listed after the usable accounts, and keeps its place in the saved order while the others are moved.
_Avoid_: Inactive, blocked account

**Disabled account**:
An account the user turned off. Listed last, below the unavailable accounts, and left out of the tray, the pooled totals, session copies and limit notifications. It still launches.
_Avoid_: Paused, hidden, archived account

**Profile**:
The folder that holds one account's Claude data.
_Avoid_: User data dir, data folder

**Quit Claude**:
Ending every Claude process of one account, including a Claude that is hidden in its own tray.
_Avoid_: Close, exit, kill

## Sessions

**Session**:
One Claude Code conversation from Claude Desktop's Code tab. Regular chats and Cowork sessions are not sessions.
_Avoid_: Chat, conversation, thread

**Transcript**:
The record of a session's messages. One transcript serves every account the session has been copied to.
_Avoid_: History, log

**Session copy**:
Making a session appear in another account as well. The source account keeps it, and both continue the same transcript.
_Avoid_: Transfer, move, migration

**Original account**:
The account a session was first created in. Its cost stays with this account, even for turns taken in a copy.
_Avoid_: Owner, source account

## Language and calendar

**Language**:
The language of clawshell's window, tray menu and notifications.
_Avoid_: Locale (in user-facing text)

**Calendar**:
The calendar clawshell writes dates in and counts months by. It also decides, except for Gregorian, the first day of the week. The digits follow the language, not the calendar.
_Avoid_: Date format, region

**Automatic calendar**:
The calendar setting that follows the language: Jalali for Persian, Hijri for Arabic, Gregorian otherwise.
_Avoid_: Default calendar, system calendar

**This month**:
The current month of the chosen calendar, the span the month's cost covers.
_Avoid_: Billing month, current period

**Billing month**:
The Gregorian month Anthropic bills extra usage by. It differs from This month whenever the calendar is not Gregorian.
_Avoid_: This month, cycle
