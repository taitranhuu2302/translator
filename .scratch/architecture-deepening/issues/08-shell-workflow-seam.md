# 08: Shell workflow seam — and start at login as its first workflow

**What to build:** The shell gets one seam and one workflow behind it. The seam is a small set of shell ports declared in the shell crate, not in the core: the login item, the popup presenter, the shortcut registry, and the shortcut route. The first workflow is start at login: read the current state, change the login item, save the preference, and on a failure roll back both the login item and the saved preference. The Tauri command shrinks to a hop to the main thread plus a call into the workflow. One named timeout serves every main-thread hop, replacing the inconsistent values in use.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The four shell ports exist and are implemented by the real shell; no workflow needs a Tauri handle to run.
- [ ] Start at login is one function over the seam, with the read → change → save → roll-back order visible in it.
- [ ] The command only hops to the main thread and maps the workflow's result.
- [ ] Every main-thread hop in the shell uses one shared timeout constant.
- [ ] Tests drive the workflow with fakes: success; the login item refuses the change; the save fails; and after each failure both the login item and the saved preference are exactly where they started.
- [ ] The packaged app still turns start at login on and off at the system level.
