# iOS foldable check (iPhone Duo simulator)

TACHYON iOS apps must fill whichever screen a foldable iPhone lights, with no
black bars, and lay out again when the device folds or unfolds
([ADR 0052](../adr/0052-the-manifest-is-the-tac-configuration.md), amendment
TAC-FOLD-IOS). The iPhone Duo simulator has two screens: the outer screen `LCD`
(1398x2034 px) and the inner screen `LCD-1` (2007x2853 px). It boots closed, on
the outer screen.

## Automated: closed, after boot

```sh
TAC_IOS_FAMILIES=iPhoneDuo TAC_IOS_DUO_DEVICE=<booted Duo UDID> node scripts/ios/foldable-test.mjs
```

The script builds two probe apps, one that turns freely and one locked to
portrait, installs each on the booted Duo and checks on the outer screen that:

- the window and the scene equal the screen bounds, and the screenshot has the
  size of the app's screen;
- every screen edge shows the app's color (no black bars);
- the page is as large as its web view, and the web view fills the window;
- the safe areas are not negative, the page starts below the status bar, and
  the page's `env(safe-area-inset-*)` values equal the web view's.

Screenshots go to `TAC_IOS_FOLD_EVIDENCE` (default
`target/ios-foldable-evidence`). `TAC_IOS_FOLD_APPS=<folder>` with
`TAC_IOS_FOLD_BUILD_ONLY=1` builds the apps without a simulator; a later run
with the same folder reuses them. The simulator cannot install an app from a
network volume, so the script always installs from a local copy.

Without `TAC_IOS_DUO_DEVICE` the script makes and boots its own Duo. With a
`TAC_IOS_FAMILIES` that does not name `iPhoneDuo` it skips.

## Manual: open and half open

No public `simctl` command moves the simulator's hinge. Until a hinge helper
exists (TAC-FOLD-IOS-HINGE), check the other postures by hand with a tool that
can move it, such as the Claude Code iOS simulator tool (`set_posture`):

1. Boot the Duo and install the free probe app and, for comparison, a copy of
   the portrait app with `UIRequiresFullScreen` added back (the old build).
2. For each app, set the posture `open`, then `half_open` (about 90 degrees),
   then `closed` again.
3. At each posture, save a screenshot of the lit screen and the app's probe
   file (`Documents/fold-probe.json` in the app's data container).
4. Pass when, at every posture, the fixed app's window equals the lit screen,
   no edge is black, and the probe's `layouts` count grew after each fold.
   Keep the screenshots with the run's logs.

Never use `simctl io screenConfig power` to imitate a fold: it does not move
the hinge, and it leaves the system shell crashed until the next boot.
