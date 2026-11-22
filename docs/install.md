# Install Neris

Neris is a menu-bar app: it has no Dock icon and no window at launch. After
installing, look for the **N mark in the menu bar**.

## Requirements

- macOS 14 (Sonoma) or later, on Apple Silicon or Intel
- An internet connection for translation; a provider key is optional

## Install

1. Download `Neris_<version>_universal.dmg` from the
   [latest release](https://github.com/taitranhuu2302/translator/releases/latest).
2. Open the DMG and drag **Neris** into **Applications**.
3. Neris is signed by the maintainer's own certificate, not by an Apple
   Developer ID, so macOS quarantines it. Remove the flag once:

   ```bash
   xattr -dr com.apple.quarantine /Applications/Neris.app
   ```

   If you would rather not use Terminal: right-click Neris in Applications,
   choose **Open**, and confirm. Either way this is a one-time step — updates
   installed by the app are not quarantined.
4. Open Neris. It asks for **Accessibility** once, which it needs to read the
   text you have selected in other apps. Grant it in System Settings → Privacy
   & Security → Accessibility. The grant survives updates.

## First run

Onboarding asks for your Language Pair (English ⇄ Vietnamese by default) and
offers to start Neris at login. A provider key is not required: the app falls
back to a keyless provider until you add one in Settings.

The four shortcuts are `⌥⇧D` (Quick Translate), `⌥⇧R` (Replace), `⌥⇧S` (Read
Aloud) and `⌥⇧E` (Translate Window). They are editable in Settings.

## Updates

Neris checks GitHub for a new Release at launch and every six hours, downloads
it in the background, and shows **Restart to Update** in the menu bar when it
is ready. It never restarts on its own. Settings → Updates has the current
version, a **Check for updates** button, and a switch to turn the automatic
check off.

If an update cannot be installed, download the new DMG from the releases page
and repeat step 2–3. Settings and history are kept.

## Uninstall

Quit Neris from the menu bar, then delete `/Applications/Neris.app`. Settings,
history and stored provider keys live under
`~/Library/Application Support/com.taitran.neris` and in the login Keychain;
delete them if you want a clean slate.
