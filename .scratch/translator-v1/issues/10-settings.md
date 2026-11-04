# 10: Settings

**What to build:** The user can shape the app to their own machine: change the Language Pair, rebind any of the four shortcuts, and see which providers are ready and which are exhausted. A shortcut that another app already owns is reported instead of quietly doing nothing.

**Blocked by:** 06

**Status:** ready-for-agent

- [ ] The Language Pair can be changed and survives relaunch
- [ ] Each of the four shortcuts can be rebound and the new binding survives relaunch
- [ ] A binding another app already owns surfaces a conflict at the moment it is set
- [ ] Provider status shows which providers have a key and which have been skipped after a quota or auth failure
- [ ] Resetting to defaults restores the four shipped shortcuts and the default pair
