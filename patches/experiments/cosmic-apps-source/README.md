# Experiment: cosmic-apps-source, COSMIC apps with faster icon lookups

Test only ([#33](https://github.com/redox-rpi/redox-rpi/issues/33)), with
`orbital-apps`. Builds `cosmic-edit` and `cosmic-files` from source, at the
commits of upstream's binary packages (`rev` in the two recipes), with
[`freedesktop-icons/0001-keep-index-theme-in-memory-remember-missing-icons.patch`](freedesktop-icons/0001-keep-index-theme-in-memory-remember-missing-icons.patch)
applied to `pop-os/freedesktop-icons` (`ab4c57b`, the version both apps
lock). The recipe clones that crate, applies the patch and hands it to cargo
as a `[patch]` in `--config`, so the app sources stay as pinned.

## Why

Pi test #272: with the COSMIC file dialog open, redoxfs used 79 % of a core
and the dialog took 20-55 s to show; typing in the editor cost redoxfs 24 %.
`redoxfs-reads` showed the icon theme index files mapped over and over
(`Cosmic/index.theme` 114 times). In freedesktop-icons, every icon lookup
that misses its cache reads (maps) the theme's `index.theme` again and then
probes every directory listed there (hicolor lists hundreds), and an icon
that was not found is looked up again after 5 seconds. On Redox every file
access is a call to the file system daemon.

The patch keeps each `index.theme` in memory after the first read and
remembers a missing icon for 10 minutes instead of 5 seconds.

## Upstream report (for the user to file)

pop-os/freedesktop-icons: "Icon lookups re-read index.theme and retry
missing icons every 5 s", with the numbers above and the patch. It is not
Redox specific; Linux hides the cost with the page cache.
