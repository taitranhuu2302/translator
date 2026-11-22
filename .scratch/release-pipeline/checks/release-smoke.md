# Release pipeline and in-app updates — manual checks

Written 2026-11-22 for tickets 01–06. The agent builds and signs; the person
opens the bundle and walks this table. Part A runs on the local build; parts B
and C need a pushed tag and two published versions.

## A. A local build cannot update itself

Build with the documented local command, then open
`target/bundle/Neris.app`:

```bash
./scripts/build.sh
```

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open Settings and scroll to **Updates**. | It shows the current version and the sentence that this build cannot update itself. **Check now** is disabled. |
| 2 | Look at the automatic-check switch. | It is visible but disabled, so a local build cannot arm a check it has no updater for. |
| 3 | Quit and relaunch. | Nothing was replaced; the bundle's timestamp is the one you built. No request to GitHub is made. |

## B. The tag produces a draft, not a release

After `./scripts/set-version.sh X.Y.Z`, a commit, and `git tag vX.Y.Z &&
git push origin main --tags`:

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open the workflow run. | It fails before building if the tag and `scripts/version.sh` disagree. With them equal, it imports the certificate, runs the suite, packages, signs and uploads. |
| 2 | Open the repository's Releases page. | A **draft** named `Neris X.Y.Z` exists with five assets: the DMG, the tarball, its `.sig`, `latest.json` and `notes.md`. Its body names the commits since the previous tag. |
| 3 | In another terminal, open the draft's app and check its version. | Settings → Updates reports `X.Y.Z`. The bundle's designated requirement still pins the local certificate (`codesign -d -r- Neris.app`). |
| 4 | While it is still a draft, open an older install and press **Check now**. | It does **not** offer the draft: `/releases/latest/download/` never points at one. |

## C. First install, update, and the two ways it can fail

Publish the draft, then:

| # | Do | Expect |
| --- | --- | --- |
| 1 | Download `Neris_X.Y.Z_universal.dmg` in a browser, drag the app into Applications, and open it. | macOS refuses it until you run `xattr -dr com.apple.quarantine /Applications/Neris.app`; after that it opens normally. |
| 2 | Grant Accessibility once and use a shortcut. | Quick Translate reads the Selection. The grant is the one the update will keep. |
| 3 | With an older install open, wait for (or force) a check. | The download is silent; the app keeps working. When it finishes, the menu bar gains **Restart to Update to X.Y.Z…** and Settings → Updates offers the same action. |
| 4 | Choose **Restart to Update**. | The app quits, relaunches as `X.Y.Z`, and Accessibility still works without re-granting. |
| 5 | Turn **Check for updates automatically** off, then relaunch. | No check runs at launch. Turning it back on checks at once rather than after six hours. |
| 6 | Disconnect the network and press **Check now**. | Settings says the check failed and gives the reason; the app keeps translating once the network returns. Nothing is left busy. |
| 7 | Publish a version whose tarball was not signed by the release key (or edit `latest.json` by hand). | The install refuses the payload and reports a failure in Settings. The app stays on the old version. |

If row 4 loses Accessibility, the certificate changed: stop, and treat it as the
certificate-rotation case in `docs/release.md`.
